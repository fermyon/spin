use super::Service;
use anyhow::Context as _;
use std::{
    path::Path,
    process::{Command, Stdio},
};

pub struct PythonService {
    child: std::process::Child,
    lock: fslock::LockFile,
}

impl PythonService {
    pub fn start(name: &str, service_definitions_path: &Path) -> anyhow::Result<Self> {
        let mut lock =
            fslock::LockFile::open(&service_definitions_path.join(format!("{name}.lock")))
                .context("failed to open service file lock")?;
        lock.lock().context("failed to obtain service file lock")?;
        let child = python()
            .arg(
                service_definitions_path
                    .join(format!("{name}.py"))
                    .display()
                    .to_string(),
            )
            // Ignore stdout
            .stdout(Stdio::null())
            .spawn()
            .context("service failed to spawn")?;
        Ok(Self { child, lock })
    }
}

impl Service for PythonService {
    fn error(&mut self) -> anyhow::Result<()> {
        let exit = self.child.try_wait()?;
        if exit.is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "process exited early",
            )
            .into());
        }
        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        let _ = self.child.kill();
        let _ = self.lock.unlock();
        Ok(())
    }
}

fn python() -> Command {
    Command::new("python3")
}
