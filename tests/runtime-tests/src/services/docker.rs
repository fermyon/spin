use super::Service;
use anyhow::{bail, Context as _};
use std::{
    path::Path,
    process::{Command, Stdio},
};

/// A docker container as a service
pub struct DockerService {
    image_name: String,
    lock: fslock::LockFile,
}

impl DockerService {
    /// Start a docker container as a service
    pub fn start(
        name: &str,
        service_definitions_path: &Path,
        config: DockerServiceConfig,
    ) -> anyhow::Result<Self> {
        let docker_file_path = service_definitions_path.join(format!("{name}.Dockerfile"));
        let image_name = format!("spin/runtime-tests/services/{name}");
        let mut lock =
            fslock::LockFile::open(&service_definitions_path.join(format!("{name}.lock")))
                .context("failed to open service file lock")?;
        lock.lock().context("failed to obtain service file lock")?;
        stop_containers(&get_running_containers(&image_name)?)?;
        build_image(&docker_file_path, &image_name)?;
        let status = Command::new("docker")
            .arg("run")
            .arg("-d")
            .arg("-p")
            .arg(format!("{}:{}", config.port, config.port))
            .arg(&image_name)
            .status()
            .context("service failed to spawn")?;
        if !status.success() {
            bail!("failed to run docker image");
        }
        Ok(Self { image_name, lock })
    }
}

impl Service for DockerService {
    fn error(&mut self) -> anyhow::Result<()> {
        anyhow::ensure!(!get_running_containers(&self.image_name)?.is_empty());
        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        stop_containers(&get_running_containers(&self.image_name)?)?;
        self.lock.unlock()?;
        Ok(())
    }
}

pub struct DockerServiceConfig {
    pub port: u16,
}

fn build_image(docker_file_path: &Path, image_name: &String) -> anyhow::Result<()> {
    let temp_dir = temp_dir::TempDir::new()
        .context("failed to produce a temporary directory to run docker in")?;
    let status = Command::new("docker")
        .arg("build")
        .arg("-f")
        .arg(&docker_file_path)
        .arg("-t")
        .arg(image_name)
        .arg(temp_dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("service failed to spawn")?;

    if !status.success() {
        bail!("failed to build docker image");
    }
    Ok(())
}

fn get_running_containers(image_name: &str) -> anyhow::Result<Vec<String>> {
    let output = Command::new("docker")
        .arg("ps")
        .arg("-q")
        .arg("--filter")
        .arg(format!("ancestor={image_name}"))
        .output()
        .context("failed to get running containers")?;
    let output = String::from_utf8(output.stdout)?;
    Ok(output.lines().map(|s| s.to_owned()).collect())
}

fn stop_containers(ids: &[String]) -> anyhow::Result<()> {
    for id in ids {
        Command::new("docker")
            .arg("stop")
            .arg(id)
            .output()
            .context("failed to stop container")?;
        let _ = Command::new("docker").arg("rm").arg(id).output();
    }
    Ok(())
}
