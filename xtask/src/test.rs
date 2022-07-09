use crate::cmd;
use crate::Result;
use crate::Shell;

impl crate::Test {
    pub fn exec(&self, sh: &Shell) -> Result<()> {
        let log_level = sh.var("RUST_LOG").unwrap_or("spin=trace".to_owned());
        sh.set_var("RUST_LOG", log_level);

        cmd!(
            sh,
            "cargo test --all --no-fail-fast -- --nocapture --include-ignored"
        )
        .run()?;

        cmd!(
            sh,
            "cargo clippy --all-targets --all-features -- -D warnings"
        )
        .run()?;

        cmd!(sh, "cargo fmt --all -- --check").run()?;

        Ok(())
    }
}
