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
    ready: bool,
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
            ready: false,
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
                let (guest_port, _) = guest
                    .split_once('/')
                    .context("guest mapping does not contain '/'")?;
                let host_port = host
                    .rsplit(':')
                    .next()
                    .expect("`rsplit` should always return one element but somehow did not");
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

    fn ready(&mut self) -> anyhow::Result<()> {
        // docker container inspect -f '{{.State.Health.Status}}'
        while !self.ready {
            let output = Command::new("docker")
                .arg("container")
                .arg("inspect")
                .arg("-f")
                // Ensure that .State.Health exists and otherwise just print that it's healthy
                .arg("{{with .State.Health}}{{.Status}}{{else}}healthy{{end}}")
                .arg(&self.container.id)
                .output()
                .context("failed to determine container health")?;
            if !output.status.success() {
                let stderr = std::str::from_utf8(&output.stderr).unwrap_or("<non-utf8>");
                bail!("docker health status check failed: {stderr}");
            }
            let output = String::from_utf8(output.stdout)?;
            match output.trim() {
                "healthy" => self.ready = true,
                "unhealthy" => bail!("docker container is unhealthy"),
                _ => std::thread::sleep(std::time::Duration::from_millis(100)),
            }
        }
        anyhow::ensure!(!get_running_containers(&self.image_name)?.is_empty());
        Ok(())
    }

    fn ports(&mut self) -> anyhow::Result<&HashMap<u16, u16>> {
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
        .arg("--health-start-period=1s")
        .arg(image_name)
        .output()
        .context("service failed to spawn")?;
    if !output.status.success() {
        bail!("failed to run docker image");
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
