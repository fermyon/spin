use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::Context;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    for test in std::fs::read_dir("tests")? {
        let test = test?;
        let temp = temp_dir::TempDir::new()?;
        log::trace!("temporary directory: {}", temp.path().display());
        if test.file_type()?.is_dir() {
            log::info!("Testing: {}", test.path().display());
            copy_manifest(&test.path(), &temp)?;
            copy_data(&test.path(), &temp)?;
            let args = get_args(&test.path())?;
            log::info!("Starting Spin...");
            let mut spin = Spin::start(&temp.path(), &args)?;
            log::info!("Started Spin...");

            let response: Response = make_http_request(&mut spin)?.parse()?;
            match response {
                Response::Ok => println!("Test passed!"),
                Response::Err(e) => println!("Test errored: {e}"),
                Response::ErrNoBody => {
                    let stderr = spin.kill()?;
                    eprintln!("Spin did not return error body (most likely due to a test panicking). stderr:\n{stderr}");
                }
            }
        } else {
            todo!("Support Spin.toml only tests")
        }
    }
    Ok(())
}

fn copy_data(test_dir: &Path, temp: &temp_dir::TempDir) -> anyhow::Result<()> {
    let dir = match std::fs::read_dir(&test_dir.join("data")) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    for item in dir {
        let item = item?;
        if item.file_type()?.is_file() {
            std::fs::copy(item.path(), temp.path().join(item.file_name()))?;
        }
    }
    Ok(())
}

fn get_args(test_dir: &Path) -> anyhow::Result<Vec<String>> {
    match std::fs::read_to_string(test_dir.join("args")) {
        Ok(s) => Ok(s.lines().map(|s| s.trim().to_owned()).collect()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e.into()),
    }
}

/// Copies the test dir's manifest file into the temporary directory
fn copy_manifest(test_dir: &Path, temp: &temp_dir::TempDir) -> anyhow::Result<()> {
    let manifest_path = test_dir.join("spin.toml");
    let manifest = std::fs::read_to_string(&manifest_path).with_context(|| {
        format!(
            "no spin.toml manifest found in test directory {}",
            test_dir.display()
        )
    })?;
    let mut table = manifest
        .parse::<toml::Table>()
        .context("could not parse the manifest as toml")?;
    for component in table["component"].as_array_mut().with_context(|| {
        format!(
            "malformed manifest '{}' does not contain 'component' array",
            manifest_path.display(),
        )
    })? {
        let source = component.get_mut("source").with_context(|| {
            format!(
                "malformed manifest '{}' does not contain 'source' string key in component",
                manifest_path.display()
            )
        })?;
        let source_str = source.as_str().with_context(|| {
            format!(
                "malformed manifest '{}' contains 'source' key that isn't a string in component",
                manifest_path.display()
            )
        })?;
        // TODO: make this more robust
        if source_str.starts_with("{{") {
            let name = &source_str[2..source_str.len() - 2];
            let path = component_path(name);
            let wasm_name = format!("{name}.wasm");
            std::fs::copy(&path, temp.path().join(&wasm_name)).with_context(|| {
                format!(
                    "failed to copy wasm file '{}' to temporary directory",
                    path.display()
                )
            })?;
            *source = toml::Value::String(wasm_name);
        }
    }
    let manifest =
        toml::to_string(&table).context("failed to re-serialize manifest as a string")?;
    std::fs::write(temp.path().join("spin.toml"), manifest)
        .context("failed to copy spin.toml manifest to temporary directory")?;
    Ok(())
}

#[derive(Debug)]
enum Response {
    Ok,
    Err(String),
    /// This happens when we panic
    ErrNoBody,
}

impl FromStr for Response {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let code = s
            .strip_prefix("HTTP/1.1 ")
            .context("malformed response does not contain `HTTP/1.1` prefix")?[..3]
            .parse::<u16>()
            .context("malformed response does not contain a status code")?;
        anyhow::ensure!(
            &s.as_bytes()[s.len() - 4..] == &b"\r\n\r\n"[..],
            r#"malformed response does not end with the expected CRLF"#
        );
        if code == 500 {
            let header_end = s
                .find("\r\n\r\n")
                .context("malformed response does not contain CRLF header separator")?;
            if header_end + 4 == s.len() {
                return Ok(Response::ErrNoBody);
            }
            let body =
                String::from_utf8(s.as_bytes()[header_end + 4..s.len() - 4].to_vec()).unwrap();
            return Ok(Response::Err(body));
        }
        if code == 200 {
            return Ok(Response::Ok);
        }
        anyhow::bail!("Could not parse HTTP raw response: {s}")
    }
}

fn make_http_request(spin: &mut Spin) -> Result<String, anyhow::Error> {
    if let Some(status) = spin.try_wait()? {
        anyhow::bail!("Spin exited early with status code {:?}", status.code());
    }
    log::debug!("Connecting to HTTP server");
    let mut connection = std::net::TcpStream::connect("127.0.0.1:3000")
        .context("could not connect to Spin web server")?;
    connection.write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1:3000\r\n\r\n")?;
    log::debug!("Awaiting response from server");
    let mut start = 0;
    let mut output = vec![0; 1024];
    for _ in 0..5 {
        let n = connection.read(&mut output[start..])?;
        start += n;
        let header_end = output.windows(4).position(|c| c == b"\r\n\r\n");
        if let Some(header_end) = header_end {
            let until_headers = String::from_utf8(output[..header_end].to_vec())
                .context("spin HTTP response headers contained non-utf8 bytes")?;
            if let Some(s) = until_headers.find("content-length: ") {
                if std::str::from_utf8(&until_headers.as_bytes()[s + 16..])
                    .unwrap()
                    .starts_with("0")
                {
                    let response_with_no_body =
                        String::from_utf8(output[..header_end + 4].to_vec()).unwrap();
                    return Ok(response_with_no_body);
                }
            }
            if let Some(_) = output[header_end + 4..]
                .windows(4)
                .find(|c| c == b"\r\n\r\n")
            {
                let response = String::from_utf8(output[..start].to_vec())
                    .context("spin HTTP response contained non-utf8 bytes")?;
                return Ok(response);
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
        if let Some(status) = spin.try_wait()? {
            anyhow::bail!("Spin exited early with status code {:?}", status.code());
        }
    }
    anyhow::bail!("did not return http response after 5 seconds")
}

struct Spin {
    process: std::process::Child,
}

impl Spin {
    fn start(dir: &Path, args: &[String]) -> Result<Self, anyhow::Error> {
        log::trace!("Starting up Spin with `spin up {}`", args.join(" "));
        let mut child = std::process::Command::new("spin")
            .arg("up")
            .current_dir(dir)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        log::debug!("Awaiting spin binary to start up");
        for _ in 0..10 {
            match std::net::TcpStream::connect("127.0.0.1:3000") {
                Ok(_) => return Ok(Self { process: child }),
                Err(e) => {
                    log::trace!("Checking that the Spin server started returned an error: {e}")
                }
            }
            if let Some(status) = child.try_wait()? {
                let mut stderr = String::new();
                let _ = child.stderr.unwrap().read_to_string(&mut stderr);
                anyhow::bail!(
                    "Spin exited early with status code {:?}\n{stderr}",
                    status.code()
                );
            }

            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        child
            .kill()
            .context("`spin up` did not start server or error and killing the process failed")?;
        let mut stderr = vec![0; 4048];
        let _ = child.stderr.take().unwrap().read(&mut stderr);
        anyhow::bail!("`spin up` did not start server or error after 5 seconds")
    }

    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.process.try_wait()
    }

    fn kill(mut self) -> anyhow::Result<String> {
        kill_process(&mut self.process);
        let mut stderr = vec![0; 4048];
        let _ = self.process.stderr.take().unwrap().read(&mut stderr);
        Ok(String::from_utf8(stderr).unwrap())
    }
}

impl Drop for Spin {
    fn drop(&mut self) {
        kill_process(&mut self.process);
    }
}

fn kill_process(process: &mut std::process::Child) {
    #[cfg(windows)]
    {
        let _ = process.kill();
    }
    #[cfg(not(windows))]
    {
        let pid = nix::unistd::Pid::from_raw(process.id() as i32);
        let _ = nix::sys::signal::kill(pid, nix::sys::signal::SIGTERM);
    }
}

fn component_path(name: &str) -> PathBuf {
    PathBuf::from("../../test-components/")
        .join(name)
        .join("component.wasm")
}
