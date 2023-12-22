use super::Service;
use anyhow::Context as _;
use std::{
    collections::HashMap,
    path::Path,
    process::{Command, Stdio},
};

pub struct PythonService {
    child: std::process::Child,
    _lock: fslock::LockFile,
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
        Ok(Self { child, _lock: lock })
    }
}

impl Service for PythonService {
    fn name(&self) -> &str {
        "python"
    }

    fn await_ready(&self) -> anyhow::Result<()> {
        Ok(())
    }

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

    fn ports(&self) -> anyhow::Result<&HashMap<u16, u16>> {
        todo!()
    }
}

impl Drop for PythonService {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn python() -> Command {
    Command::new("python3")
}
