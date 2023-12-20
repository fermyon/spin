use super::Service;
use anyhow::{bail, Context as _};
use std::{
    cell::OnceCell,
    collections::HashMap,
    path::Path,
    process::{Command, Stdio},
};

/// A docker container as a service
pub struct DockerService {
    image_name: String,
    container: Container,
    // We declare lock after container so that the lock is dropped after the container is
    _lock: fslock::LockFile,
    ports: OnceCell<HashMap<u16, u16>>,
}

impl DockerService {
    /// Start a docker container as a service
    pub fn start(name: &str, service_definitions_path: &Path) -> anyhow::Result<Self> {
        let docker_file_path = service_definitions_path.join(format!("{name}.Dockerfile"));
        let image_name = format!("spin/runtime-tests/services/{name}");
        let mut lock =
            fslock::LockFile::open(&service_definitions_path.join(format!("{name}.lock")))
                .context("failed to open service file lock")?;
        lock.lock().context("failed to obtain service file lock")?;

        stop_containers(&get_running_containers(&image_name)?)?;
        build_image(&docker_file_path, &image_name)?;
        let container = run_container(&image_name)?;

        Ok(Self {
            image_name,
            container,
            _lock: lock,
            ports: OnceCell::new(),
        })
    }
}

struct Container {
    id: String,
}

impl Container {
    fn get_ports(&self) -> anyhow::Result<HashMap<u16, u16>> {
        let output = Command::new("docker")
            .arg("port")
            .arg(&self.id)
            .output()
            .context("docker failed to fetch ports")?;
        if !output.status.success() {
            bail!("failed to run fetch ports for docker container");
        }
        let output = String::from_utf8(output.stdout)?;
        output
            .lines()
            .map(|s| {
                // 3306/tcp -> 0.0.0.0:32770
                let s = s.trim();
                let (guest, host) = s
                    .split_once(" -> ")
                    .context("failed to parse port mapping")?;
                let (guest_port, _) = guest.split_once('/').context("TODO")?;
                let host_port = host.rsplit(':').next().context("TODO")?;
                Ok((guest_port.parse()?, host_port.parse()?))
            })
            .collect()
    }
}

impl Drop for Container {
    fn drop(&mut self) {
        let _ = stop_containers(&[std::mem::take(&mut self.id)]);
    }
}

impl Service for DockerService {
    fn name(&self) -> &str {
        "docker"
    }

    fn error(&mut self) -> anyhow::Result<()> {
        anyhow::ensure!(!get_running_containers(&self.image_name)?.is_empty());
        Ok(())
    }

    fn ports(&self) -> anyhow::Result<&HashMap<u16, u16>> {
        match self.ports.get() {
            Some(p) => Ok(p),
            None => {
                let ports = self.container.get_ports()?;
                Ok(self.ports.get_or_init(|| ports))
            }
        }
    }
}

fn build_image(docker_file_path: &Path, image_name: &String) -> anyhow::Result<()> {
    let temp_dir = temp_dir::TempDir::new()
        .context("failed to produce a temporary directory to run docker in")?;
    let status = Command::new("docker")
        .arg("build")
        .arg("-f")
        .arg(docker_file_path)
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

fn run_container(image_name: &str) -> anyhow::Result<Container> {
    let output = Command::new("docker")
        .arg("run")
        .arg("-d")
        .arg("-P")
        .arg(image_name)
        .output()
        .context("service failed to spawn")?;
    if !output.status.success() {
        bail!("failed to run docker image");
    }
    // TODO: figure out how we get rid of this hack
    // This is currently necessary because mysql takes a while to start up
    if image_name.contains("mysql") {
        std::thread::sleep(std::time::Duration::from_secs(15));
    }
    let output = String::from_utf8(output.stdout)?;
    let id = output.trim().to_owned();
    Ok(Container { id })
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
