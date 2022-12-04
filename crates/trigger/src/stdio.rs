use std::{collections::HashSet, fs::File, path::PathBuf};

use anyhow::Context;

use crate::{TriggerHooks, SPIN_HOME};

/// Which components should have their logs followed on stdout/stderr.
#[derive(Clone, Debug)]
pub enum FollowComponents {
    /// Only the specified components should have their logs followed.
    Named(HashSet<String>),
    /// All components should have their logs followed.
    All,
}

impl FollowComponents {
    /// Whether a given component should have its logs followed on stdout/stderr.
    pub fn should_follow(&self, component_id: &str) -> bool {
        match self {
            Self::All => true,
            Self::Named(ids) => ids.contains(component_id),
        }
    }
}

impl Default for FollowComponents {
    fn default() -> Self {
        Self::All
    }
}

/// Defines where to write logs
pub enum LogDestination {
    /// Write logs to stdout/stderr
    Std,
    /// Write logs to files in the directory
    Dir(Option<PathBuf>),
}

/// Implements TriggerHooks, writing logs to a log file or stdout/stderr
pub struct StdioLoggingTriggerHooks {
    follow_components: FollowComponents,
    log: LogDestination,
}

impl StdioLoggingTriggerHooks {
    pub fn new(follow_components: FollowComponents, log: LogDestination) -> Self {
        Self {
            follow_components,
            log,
        }
    }

    fn component_log_writer(
        &self,
        builder: &mut spin_core::StoreBuilder,
        component_id: &str,
    ) -> anyhow::Result<()> {
        if self.follow_components.should_follow(component_id) {
            match &self.log {
                LogDestination::Std => {
                    builder.stdout_pipe(std::io::stdout());
                    builder.stderr_pipe(std::io::stderr());
                }
                LogDestination::Dir(log_path) => {
                    let log_path = log_path
                        .as_deref()
                        .expect("log should have been initialized in app_loaded");

                    let sanitized_component_id = sanitize_filename::sanitize(component_id);
                    let out_path = log_path.join(format!("{sanitized_component_id}_stdout.txt"));
                    let err_path = log_path.join(format!("{sanitized_component_id}_stderr.txt"));

                    builder.stdout_pipe(log_file(&out_path)?);
                    builder.stderr_pipe(log_file(&err_path)?);
                }
            };
        }

        Ok(())
    }

    fn create_log_dir(&mut self, app: &spin_app::App) -> anyhow::Result<()> {
        let app_name: &str = app.require_metadata("name")?;

        // Set default log_dir (if not explicitly passed)
        let log_dir = match self.log {
            LogDestination::Std => {
                panic!("Log dir creation shouldn't be called when destination is stdout/stderr")
            }
            LogDestination::Dir(ref mut dir) => {
                dir.get_or_insert_with(|| default_log_dir(app_name))
            }
        };

        // Ensure log dir exists
        std::fs::create_dir_all(&log_dir)
            .with_context(|| format!("Failed to create log dir {log_dir:?}"))?;

        Ok(())
    }
}

impl TriggerHooks for StdioLoggingTriggerHooks {
    fn app_loaded(&mut self, app: &spin_app::App) -> anyhow::Result<()> {
        if let LogDestination::Dir(_dir) = &self.log {
            self.create_log_dir(app)?;
        }

        Ok(())
    }

    fn component_store_builder(
        &self,
        component: spin_app::AppComponent,
        builder: &mut spin_core::StoreBuilder,
    ) -> anyhow::Result<()> {
        self.component_log_writer(builder, component.id())
    }
}

fn default_log_dir(app_name: &str) -> PathBuf {
    let parent_dir = match dirs::home_dir() {
        Some(home) => home.join(SPIN_HOME),
        None => PathBuf::new(), // "./"
    };
    let sanitized_app = sanitize_filename::sanitize(app_name);

    parent_dir.join(sanitized_app).join("logs")
}

fn log_file(file_path: &PathBuf) -> anyhow::Result<File> {
    File::options()
        .create(true)
        .append(true)
        .open(file_path)
        .with_context(|| format!("Failed to open log file {file_path:?}"))
}
