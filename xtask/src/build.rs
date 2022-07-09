use crate::cmd;
use crate::Result;
use crate::Shell;

impl crate::Build {
    pub fn exec(&self, sh: &Shell) -> Result<()> {
        cmd!(sh, "cargo build --release").run()
    }
}
