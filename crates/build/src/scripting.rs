// TODO: subset of spin_templates::scripting

use is_terminal::IsTerminal;

pub(crate) fn register_functions(engine: &mut rhai::Engine) {
    engine.register_fn("ask_yn", ask_yn);
    engine.register_fn("exec", exec);
    engine.register_fn("interactive", interactive);
    engine
        .register_type::<CommandOutput>()
        .register_get("program_found", CommandOutput::program_found)
        .register_get("exit_code", CommandOutput::exit_code)
        .register_get("stdout", CommandOutput::stdout)
        .register_get("stderr", CommandOutput::stderr);
}

// Functions and types to be injected into the scripting engine

fn exec(
    command: rhai::ImmutableString,
    args: rhai::Array,
) -> Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
    let command = command.to_string();
    let args = args.iter().map(|item| item.to_string()).collect::<Vec<_>>();
    let outputr = std::process::Command::new(command).args(args).output();

    let output = match outputr {
        Ok(output) => CommandOutput {
            program_found: true,
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        },
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => CommandOutput {
                program_found: false,
                exit_code: -2,
                stdout: "".to_owned(),
                stderr: "".to_owned(),
            },
            _ => return Err(Box::<rhai::EvalAltResult>::from(e.to_string())),
        },
    };

    Ok(rhai::Dynamic::from(output))
}

fn ask_yn(text: rhai::ImmutableString) -> bool {
    if !std::io::stderr().is_terminal() {
        eprintln!("Answering 'no' to '{text}'");
        return false;
    }
    crate::interaction::confirm(text.as_ref()).unwrap_or(false)
}

fn interactive() -> bool {
    std::io::stderr().is_terminal()
}

#[derive(Clone, Debug)]
struct CommandOutput {
    program_found: bool,
    exit_code: i32,
    stdout: String,
    stderr: String,
}

impl CommandOutput {
    fn program_found(&mut self) -> bool {
        self.program_found
    }

    fn exit_code(&mut self) -> i64 {
        self.exit_code.into()
    }

    fn stdout(&mut self) -> String {
        self.stdout.clone()
    }

    fn stderr(&mut self) -> String {
        self.stderr.clone()
    }
}
