use clap::CommandFactory;
use shell_completion::{CompletionInput, CompletionSet};
use spin_cli::SpinApp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let input = shell_completion::BashCompletionInput::from_env().unwrap();
    complete(input).await.suggest();
    Ok(())
}

async fn complete(input: impl CompletionInput) -> Vec<String> {
    match input.arg_index() {
        0 => unreachable!(),
        1 => complete_spin_commands(input),
        _ => {
            let sc = input.args()[1].to_owned();
            complete_spin_subcommand(&sc, input).await
        }
    }
}

fn complete_spin_commands(input: impl CompletionInput) -> Vec<String> {
    let command = SpinApp::command();

    // --help and --version don't show up as options so this doesn't complete them,
    // but I'm not going to lose much sleep over that.

    // TODO: this doesn't currently offer plugin names as completions.

    let candidates = command
        .get_subcommands()
        .filter(|c| !c.is_hide_set())
        .map(|c| c.get_name());
    input.complete_subcommand(candidates)
}

async fn complete_spin_subcommand(name: &str, input: impl CompletionInput) -> Vec<String> {
    let command = SpinApp::command().to_owned();
    let Some(subcommand) = command.find_subcommand(name) else {
        return vec![]; // TODO: is there a way to hand off to a plugin?
    };
    let subcommand = subcommand.to_owned();

    if subcommand.has_subcommands() {
        // TODO: make this properly recursive instead of hardwiring to 2 levels of subcommand
        if input.arg_index() <= 2 {
            let sub_subcommands = subcommand
                .get_subcommands()
                .filter(|c| !c.is_hide_set())
                .map(|c| c.get_name());
            return input.complete_subcommand(sub_subcommands);
        } else {
            let ssc = input.args()[2];
            let Some(sub_subcommand) = subcommand.find_subcommand(ssc) else {
                return vec![];
            };
            let sub_subcommand = sub_subcommand.to_owned();
            return complete_cmd(sub_subcommand, 2, input).await;
        }
    }

    complete_cmd(subcommand, 1, input).await
}

async fn complete_cmd(
    cmd: clap::Command<'_>,
    depth: usize,
    input: impl CompletionInput,
) -> Vec<String> {
    let subcommand_key = input.args()[1..(depth + 1)].join("-");
    let forwards_args = ["up", "build", "watch"].contains(&(subcommand_key.as_str())); // RUST Y U TREAT ME THIS WAY

    // Strategy:
    // If the PREVIOUS word was a PARAMETERISED option:
    // - Figure out possible values and offer them
    // Otherwise (i.e. if the PREVIOUS word was a NON-OPTION (did not start with '-'), or a UNARY option):
    // - If ALL positional parameters are satisfied:
    //   - Offer the options
    // - Otherwise:
    //   - If the current word is EMPTY and the NEXT available positional is completable:
    //     - Offer the NEXT positional
    //   - If the current word is EMPTY and the NEXT positional is NON-COMPLETABLE:
    //     - Offer the options
    //   - If the current word is NON-EMTPY:
    //     - Offer the options AND the NEXT positional if completable

    // IMPORTANT: this strategy *completely breaks* for `spin up` because it technically has
    // an infinitely repeatable positional parameter for `trigger_args`.  Also `build` and
    // `watch` which have `up_args`.

    let app = SpinApp::command();
    let mut args = cmd
        .get_arguments()
        .map(|a| a.to_owned())
        .collect::<Vec<_>>();
    if forwards_args {
        let trigger_args = app
            .find_subcommand("trigger")
            .unwrap()
            .find_subcommand("http")
            .unwrap()
            .get_arguments()
            .map(|a| a.to_owned())
            .collect::<Vec<_>>();
        args.extend(trigger_args.into_iter());

        if subcommand_key != "up" {
            let up_args = app
                .find_subcommand("up")
                .unwrap()
                .get_arguments()
                .map(|a| a.to_owned())
                .collect::<Vec<_>>();
            args.extend(up_args.into_iter());
            args.retain(|a| a.get_name() != "up-args");
        }
        args.retain(|a| a.get_name() != "trigger-args");
    }
    let prev_arg = args.iter().find(|o| o.is_match(input.previous_word()));

    // Are we in a position of completing a value-ful flag?
    if let Some(prev_option) = prev_arg {
        if prev_option.is_takes_value_set() {
            let complete_with = CompleteWith::infer(&subcommand_key, prev_option);
            return complete_with.completions(input).await;
        }
    }

    // No: previous word was not a flag, or was unary (or was unknown)

    // Are all positional parameters satisfied?
    let num_positionals = if forwards_args {
        0
    } else {
        cmd.get_positionals().count()
    };
    let first_unfulfilled_positional = if num_positionals == 0 {
        None
    } else {
        let mut num_positionals_provided = 0;
        let in_progress = !(input.args().last().unwrap().is_empty()); // safe to unwrap because we are deep in subcommanery here
        let mut provided = input.args().into_iter().skip(depth + 1);
        let mut prev: Option<&str> = None;
        let mut last_was_positional = false;
        loop {
            let Some(cur) = provided.next() else {
                if in_progress && last_was_positional {
                    num_positionals_provided -= 1;
                }
                break;
            };

            if cur.is_empty() {
                continue;
            }

            let is_cur_positional = if cur.starts_with('-') {
                false
            } else {
                // It might be a positional or it might be governed by a flag
                let is_governed_by_prev = match prev {
                    None => false,
                    Some(p) => {
                        let matching_opt = cmd
                            .get_arguments()
                            .find(|a| a.long_and_short().contains(&p.to_string()));
                        match matching_opt {
                            None => false, // the previous thing was not an option, so cannot govern
                            Some(o) => o.is_takes_value_set(),
                        }
                    }
                };
                !is_governed_by_prev
            };

            if is_cur_positional {
                num_positionals_provided += 1;
            }

            last_was_positional = is_cur_positional;
            prev = Some(cur);
        }
        cmd.get_positionals().nth(num_positionals_provided)
    };

    match first_unfulfilled_positional {
        Some(arg) => {
            let complete_with = CompleteWith::infer(&subcommand_key, arg);
            complete_with.completions(input).await
        }
        None => {
            // TODO: consider positionals
            let all_args: Vec<_> = args.iter().flat_map(|o| o.long_and_short()).collect();
            input.complete_subcommand(all_args.iter().map(|s| s.as_str()))
        }
    }
}

trait ArgInfo {
    fn long_and_short(&self) -> Vec<String>;

    fn is_match(&self, text: &str) -> bool {
        self.long_and_short().contains(&text.to_string())
    }
}

impl<'a> ArgInfo for clap::Arg<'a> {
    fn long_and_short(&self) -> Vec<String> {
        let mut result = vec![];
        if let Some(c) = self.get_short() {
            result.push(format!("-{c}"));
        }
        if let Some(s) = self.get_long() {
            result.push(format!("--{s}"));
        }
        result
    }
}

enum CompleteWith {
    File,
    Directory,
    Template,
    KnownPlugin,
    InstalledPlugin,
    None,
}

impl CompleteWith {
    fn infer(subcommand_key: &str, governing_arg: &clap::Arg) -> Self {
        match governing_arg.get_value_hint() {
            clap::ValueHint::FilePath => CompleteWith::File,
            clap::ValueHint::DirPath => CompleteWith::Directory,
            _ => Self::infer_from_names(subcommand_key, governing_arg.get_name()),
        }
    }

    fn infer_from_names(subcommand_key: &str, arg_name: &str) -> Self {
        match (subcommand_key, arg_name) {
            ("add", "template-id") => Self::Template,
            ("new", "template-id") => Self::Template,
            ("plugins-install", spin_cli::opts::PLUGIN_NAME_OPT) => Self::KnownPlugin,
            ("plugins-uninstall", "name") => Self::InstalledPlugin,
            ("plugins-upgrade", "name") => Self::InstalledPlugin,
            ("templates-uninstall", "template-id") => Self::Template,
            _ => Self::None,
        }
    }

    async fn completions(&self, input: impl CompletionInput) -> Vec<String> {
        match self {
            Self::File => input.complete_file(),
            Self::Directory => input.complete_directory(),
            Self::Template => input.complete_text(templates().await),
            Self::KnownPlugin => input.complete_text(known_plugins().await),
            Self::InstalledPlugin => input.complete_text(installed_plugins().await),
            Self::None => vec![],
        }
    }
}

async fn templates() -> Vec<String> {
    if let Ok(mgr) = spin_templates::TemplateManager::try_default() {
        if let Ok(list) = mgr.list().await {
            return list
                .templates
                .into_iter()
                .map(|t| t.id().to_string())
                .collect();
        }
    }
    vec![]
}

async fn known_plugins() -> Vec<String> {
    if let Ok(mgr) = spin_plugins::manager::PluginManager::try_default() {
        if let Ok(manifests) = mgr.store().catalogue_manifests() {
            return manifests.into_iter().map(|m| m.name()).collect();
        }
    }
    vec![]
}

async fn installed_plugins() -> Vec<String> {
    if let Ok(mgr) = spin_plugins::manager::PluginManager::try_default() {
        if let Ok(manifests) = mgr.store().installed_manifests() {
            return manifests.into_iter().map(|m| m.name()).collect();
        }
    }
    vec![]
}

trait CompletionInputExt {
    fn complete_text(&self, options: Vec<String>) -> Vec<String>;
}

impl<T: CompletionInput> CompletionInputExt for T {
    fn complete_text(&self, options: Vec<String>) -> Vec<String> {
        self.complete_subcommand(options.iter().map(|s| s.as_str()))
    }
}
