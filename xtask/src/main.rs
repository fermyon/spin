use xflags::xflags;
use xshell::cmd;
use xshell::Result;
use xshell::Shell;

mod build;
mod test;
mod test_sdk;

fn main() -> xflags::Result<()> {
    let cmd = match Task::from_env() {
        Ok(cmd) => cmd,
        Err(_) => {
            println!("{}", Task::HELP);
            return Ok(());
        }
    };

    cmd.run().map_err(|e| xflags::Error::new(e.to_string()))?;
    Ok(())
}

xflags! {
    // xtask for spin
    cmd task {
        // run test
        cmd test {}
        // run build
        cmd build {}
        // test sdks
        cmd test-sdk {
            // go sdk
            optional --go
        }
    }
}

impl Task {
    fn run(&self) -> Result<()> {
        let shell = Shell::new()?;
        self.subcommand.exec(&shell)?;
        Ok(())
    }
}

impl TaskCmd {
    fn exec(&self, sh: &Shell) -> Result<()> {
        match self {
            Self::Build(b) => {
                b.exec(sh)?;
            }
            Self::Test(t) => {
                t.exec(sh)?;
            }
            Self::TestSdk(ts) => {
                ts.exec(sh)?;
            }
        }
        Ok(())
    }
}
