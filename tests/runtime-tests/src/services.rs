use std::{
    collections::HashMap,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{bail, Context};

pub fn start_services(test_path: &Path) -> anyhow::Result<Services> {
    let services_config_path = test_path.join("services");
    let children = if services_config_path.exists() {
        let services = std::fs::read_to_string(&services_config_path)
            .context("could not read services file")?;
        let required_services = services.lines().filter_map(|s| {
            let s = s.trim();
            (!s.is_empty()).then_some(s)
        });
        // TODO: make this more robust so that it is not just assumed where the services definitions are
        let service_definitions_path = test_path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("services");
        let service_definitions = std::fs::read_dir(&service_definitions_path)?
            .into_iter()
            .map(|d| {
                let d = d?;
                if !d.file_type()?.is_file() {
                    bail!("directories are not allowed in the service definitions directory")
                }
                let file_name = d.file_name();
                let file_name = file_name.to_str().unwrap();
                let (file_name, file_extension) = file_name
                    .find('.')
                    .map(|i| (&file_name[..i], &file_name[i + 1..]))
                    .context("service definition did not have an extension")?;
                Ok((file_name.to_owned(), file_extension.to_owned()))
            })
            .collect::<anyhow::Result<HashMap<_, _>>>()?;
        let mut services = Vec::new();
        for required_service in required_services {
            let service_definition_extension = service_definitions.get(required_service);
            let child = match service_definition_extension.map(|s| s.as_str()) {
                Some("py") => {
                    let mut lock = fslock::LockFile::open(
                        &service_definitions_path.join(format!("{required_service}.lock")),
                    )
                    .context("failed to open service file lock")?;
                    lock.lock().context("failed to obtain service file lock")?;
                    let child = python()
                        .arg(
                            service_definitions_path
                                .join(format!("{required_service}.py"))
                                .display()
                                .to_string(),
                        )
                        // Ignore stdout
                        .stdout(Stdio::null())
                        .spawn()
                        .context("service failed to spawn")?;
                    (child, Some(lock))
                }
                Some("docker-compose.yml") => {
                    let child = Command::new("docker-compose")
                        .arg("-f")
                        .arg(
                            service_definitions_path
                                .join(format!("{required_service}.docker-compose.yml"))
                                .display()
                                .to_string(),
                        )
                        .arg("up")
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                        .context("service failed to spawn")?;
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    (child, None)
                }
                _ => bail!("unsupported service found: {required_service}"),
            };
            services.push(child);
        }
        services
    } else {
        Vec::new()
    };

    Ok(Services { children })
}

fn python() -> Command {
    Command::new("python3")
}

pub struct Services {
    children: Vec<(std::process::Child, Option<fslock::LockFile>)>,
}

impl Services {
    pub fn error(&mut self) -> std::io::Result<()> {
        for (child, _) in &mut self.children {
            let exit = child.try_wait()?;
            if exit.is_some() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "process exited early",
                ));
            }
        }
        Ok(())
    }
}

impl Drop for Services {
    fn drop(&mut self) {
        for (child, lock) in &mut self.children {
            let _ = child.kill();
            if let Some(lock) = lock {
                let _ = lock.unlock();
            }
        }
    }
}
