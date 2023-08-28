use std::sync::{Arc, Mutex};

use watchexec::action::Outcome;

const PREVENT_CLEAR: bool = true;
const ALLOW_CLEAR: bool = false;

/// Tracks the current state of spin watch. Handles transitioning states and converting effects
/// into outcomes based on the current state.
#[derive(Debug, Clone)]
pub struct WatchState {
    state: Arc<Mutex<State>>,
    clear: bool,
    skip_build: bool,
}

impl WatchState {
    pub fn new(skip_build: bool, clear: bool) -> Self {
        let initial_state = Arc::new(Mutex::new(match skip_build {
            false => State::Building,
            true => State::Running,
        }));
        Self {
            state: initial_state,
            clear,
            skip_build,
        }
    }

    /// Get the current state of the watch command.
    pub fn get_state(&self) -> State {
        *self.state.lock().unwrap()
    }

    /// Based on the given effect return the correct outcome. Transition the internal state of the
    /// watch command if necessary.
    pub fn handle(&self, effect: Effect) -> Outcome {
        let mut state = self.state.lock().unwrap();
        tracing::debug!("handling effect {:?} in current state {:?}", effect, *state);
        // Note that outcomes are wrapped in `Outcome::if_running` to protect us from the possibility
        // that our `WatchState` is out of sync with the state of Watchexec. See more details here:
        // https://docs.rs/watchexec/latest/watchexec/action/enum.Outcome.html
        let outcome = match (effect, *state) {
            (Effect::Exit, _) => {
                Outcome::if_running(Outcome::both(stop_outcome(), Outcome::Exit), Outcome::Exit)
            }
            (Effect::ChildProcessFailed, _) => {
                Outcome::if_running(stop_outcome(), Outcome::DoNothing)
            }
            (Effect::ChildProcessCompleted, State::Building) => {
                *state = State::Running;
                self.restart_outcome(PREVENT_CLEAR)
            }
            (Effect::ChildProcessCompleted, State::Running) => {
                *state = State::Building;
                self.restart_outcome(PREVENT_CLEAR)
            }
            (Effect::ChildProcessCompleted, State::WaitingForSpinUpToExit) => {
                *state = State::Building;
                self.restart_outcome(PREVENT_CLEAR)
            }
            (Effect::ManifestChange, State::Building) => self.restart_outcome(ALLOW_CLEAR),
            (Effect::ManifestChange, State::WaitingForSpinUpToExit) => Outcome::DoNothing,
            (Effect::ManifestChange, State::Running) => {
                if !self.skip_build {
                    *state = State::WaitingForSpinUpToExit;
                    Outcome::if_running(stop_outcome(), Outcome::DoNothing)
                } else {
                    self.restart_outcome(ALLOW_CLEAR)
                }
            }
            (Effect::SourceChange, State::Building) => self.restart_outcome(ALLOW_CLEAR),
            (Effect::SourceChange, State::WaitingForSpinUpToExit) => Outcome::DoNothing,
            (Effect::SourceChange, State::Running) => {
                if !self.skip_build {
                    *state = State::WaitingForSpinUpToExit;
                    Outcome::if_running(stop_outcome(), Outcome::DoNothing)
                } else {
                    Outcome::DoNothing
                }
            }
            (Effect::ArtifactChange, State::Building) => Outcome::DoNothing,
            (Effect::ArtifactChange, State::WaitingForSpinUpToExit) => Outcome::DoNothing,
            (Effect::ArtifactChange, State::Running) => {
                *state = State::WaitingForSpinUpToExit;
                Outcome::if_running(stop_outcome(), Outcome::DoNothing)
            }
            (Effect::DoNothing, State::WaitingForSpinUpToExit) => Outcome::DoNothing,
            (Effect::DoNothing, _) => Outcome::if_running(Outcome::DoNothing, Outcome::Start),
        };
        tracing::debug!("now in {:?} state with outcome {:?}", *state, outcome);
        outcome
    }

    fn restart_outcome(&self, prevent_clear: bool) -> Outcome {
        let should_clear = !prevent_clear && self.clear;
        Outcome::sequence(
            [
                Outcome::if_running(stop_outcome(), Outcome::DoNothing),
                match should_clear {
                    true => Outcome::Clear,
                    false => Outcome::DoNothing,
                },
                Outcome::Start,
            ]
            .into_iter(),
        )
    }
}

// We prefer to stop `spin up` with Ctrl+C (Interrupt) so that it cleans up properly,
// but this may depend on the OS.
fn stop_outcome() -> Outcome {
    #[cfg(unix)]
    let stop = Outcome::Signal(watchexec::signal::source::MainSignal::Interrupt);
    #[cfg(not(unix))]
    let stop = Outcome::Stop;
    stop
}

/// A state that the watch command can be in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    Building,
    Running,
    WaitingForSpinUpToExit,
}

/// An effect is parsed from the events of an action and results in an outcome.
///
/// The variants are ordered by highest to lowest precedence so that they can be sorted. When an
/// action has multiple events (this occurs when events are debounced) they will all produce
/// effects. The highest precedence effect will be chosen to produce the outcome.
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub enum Effect {
    /// Exit spin watch
    Exit,
    /// Either `spin build` or `spin up` failed to run
    ChildProcessFailed,
    /// Either `spin build` or `spin up` has completed
    ChildProcessCompleted,
    /// Changes have been made to the application manifest
    ManifestChange,
    /// Changes have been made to the application source code
    SourceChange,
    /// Changes have been made to an application artifact
    ArtifactChange,
    /// A default option that maps to doing nothing
    DoNothing,
}

/// Effects handles the logic of choosing between multiple effects.
#[derive(Debug)]
pub struct Effects(Vec<Effect>);

impl Effects {
    pub fn new() -> Self {
        Effects(vec![Effect::DoNothing])
    }

    pub fn add(&mut self, effect: Effect) {
        self.0.push(effect);
    }

    pub fn reduce(&mut self) -> Effect {
        let effect = *self.0.iter().min().unwrap();
        tracing::debug!("effects: {:?}", self.0);
        tracing::debug!("reduced {} effects to {:?}", self.0.len(), effect);
        effect
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_state() {
        // Sm starts building
        let sm = WatchState::new(false, true);
        assert_eq!(State::Building, sm.get_state());

        // Artifacts are modified while building and nothing changes
        assert_eq!(Outcome::DoNothing, sm.handle(Effect::ArtifactChange));

        // Finishes building and doesn't clear screen
        assert_eq!(
            Outcome::both(
                Outcome::both(
                    Outcome::if_running(stop_outcome(), Outcome::DoNothing),
                    Outcome::DoNothing
                ),
                Outcome::Start
            ),
            sm.handle(Effect::ChildProcessCompleted)
        );
        assert_eq!(State::Running, sm.get_state());

        // Source change waits for `spin up` to exit...
        assert_eq!(
            Outcome::if_running(stop_outcome(), Outcome::DoNothing),
            sm.handle(Effect::SourceChange)
        );
        assert_eq!(State::WaitingForSpinUpToExit, sm.get_state());

        // ...and when it does, kicks off the build
        assert_eq!(
            Outcome::both(
                Outcome::both(
                    Outcome::if_running(stop_outcome(), Outcome::DoNothing),
                    Outcome::DoNothing
                ),
                Outcome::Start
            ),
            sm.handle(Effect::ChildProcessCompleted)
        );
        assert_eq!(State::Building, sm.get_state());

        // Build fails and it halts there
        assert_eq!(
            Outcome::if_running(stop_outcome(), Outcome::DoNothing),
            sm.handle(Effect::ChildProcessFailed)
        );
        assert_eq!(State::Building, sm.get_state());
    }

    #[test]
    fn test_watch_state_with_skip_build() {
        // Sm starts in running
        let sm = WatchState::new(true, false);
        assert_eq!(State::Running, sm.get_state());

        // Source is modified while running and nothing changes
        assert_eq!(Outcome::DoNothing, sm.handle(Effect::SourceChange));

        // Manifest change restarts server and doesn't clear screen (turned off)
        assert_eq!(
            Outcome::both(
                Outcome::both(
                    Outcome::if_running(stop_outcome(), Outcome::DoNothing),
                    Outcome::DoNothing
                ),
                Outcome::Start
            ),
            sm.handle(Effect::ManifestChange)
        );
        assert_eq!(State::Running, sm.get_state());

        // Running server fails and it halts there
        assert_eq!(
            Outcome::if_running(stop_outcome(), Outcome::DoNothing),
            sm.handle(Effect::ChildProcessFailed)
        );
        assert_eq!(State::Running, sm.get_state());
    }

    #[test]
    fn test_effects_reduces_properly() {
        let mut e1 = Effects::new();
        e1.add(Effect::DoNothing);
        assert_eq!(Effect::DoNothing, e1.reduce());

        let mut e2 = Effects::new();
        e2.add(Effect::ChildProcessCompleted);
        e2.add(Effect::ChildProcessFailed);
        e2.add(Effect::ArtifactChange);
        assert_eq!(Effect::ChildProcessFailed, e2.reduce());

        let mut e3 = Effects::new();
        e3.add(Effect::ManifestChange);
        e3.add(Effect::SourceChange);
        e3.add(Effect::Exit);
        assert_eq!(Effect::Exit, e3.reduce());
    }
}
