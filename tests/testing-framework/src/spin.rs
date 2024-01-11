use anyhow::Context;

use crate::{io::OutputStream, Runtime};
use std::{
    path::Path,
    process::{Command, Stdio},
};

/// A wrapper around a running Spin instance
pub struct Spin {
    process: std::process::Child,
    #[allow(dead_code)]
    stdout: OutputStream,
    stderr: OutputStream,
    io_mode: IoMode,
}

impl Spin {
    pub fn start(
        spin_binary_path: &Path,
        current_dir: &Path,
        spin_up_args: Vec<impl AsRef<std::ffi::OsStr>>,
        mode: SpinMode,
    ) -> anyhow::Result<Self> {
        match mode {
            SpinMode::Http => Self::start_http(spin_binary_path, current_dir, spin_up_args),
            SpinMode::Redis => Self::start_redis(spin_binary_path, current_dir, spin_up_args),
        }
    }

    /// Start Spin in `current_dir` using the binary at `spin_binary_path`
    pub fn start_http(
        spin_binary_path: &Path,
        current_dir: &Path,
        spin_up_args: Vec<impl AsRef<std::ffi::OsStr>>,
    ) -> anyhow::Result<Self> {
        let port = get_random_port()?;
        let mut child = Command::new(spin_binary_path)
            .arg("up")
            .current_dir(current_dir)
            .args(["--listen", &format!("127.0.0.1:{port}")])
            .args(spin_up_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdout = OutputStream::new(child.stdout.take().unwrap());
        let stderr = OutputStream::new(child.stderr.take().unwrap());
        log::debug!("Awaiting spin binary to start up on port {port}...");
        let mut spin = Self {
            process: child,
            stdout,
            stderr,
            io_mode: IoMode::Http(port),
        };
        let start = std::time::Instant::now();
        loop {
            match std::net::TcpStream::connect(format!("127.0.0.1:{port}")) {
                Ok(_) => {
                    log::debug!("Spin started on port {port}.");
                    return Ok(spin);
                }
                Err(e) => {
                    let stderr = spin.stderr.output_as_str().unwrap_or("<non-utf8>");
                    log::trace!("Checking that the Spin server started returned an error: {e}");
                    log::trace!("Current spin stderr = '{stderr}'");
                }
            }
            if let Some(status) = spin.try_wait()? {
                anyhow::bail!(
                    "Spin exited early with status code {:?}\n{}{}",
                    status.code(),
                    spin.stdout.output_as_str().unwrap_or("<non-utf8>"),
                    spin.stderr.output_as_str().unwrap_or("<non-utf8>")
                );
            }

            if start.elapsed() > std::time::Duration::from_secs(2 * 60) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        anyhow::bail!(
            "`spin up` did not start server or error after two minutes. stderr:\n\t{}",
            spin.stderr.output_as_str().unwrap_or("<non-utf8>")
        )
    }

    pub fn start_redis(
        spin_binary_path: &Path,
        current_dir: &Path,
        spin_up_args: Vec<impl AsRef<std::ffi::OsStr>>,
    ) -> anyhow::Result<Self> {
        let mut child = Command::new(spin_binary_path)
            .arg("up")
            .current_dir(current_dir)
            .args(spin_up_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdout = OutputStream::new(child.stdout.take().unwrap());
        let stderr = OutputStream::new(child.stderr.take().unwrap());
        let mut spin = Self {
            process: child,
            stdout,
            stderr,
            io_mode: IoMode::Redis,
        };
        // TODO this is a hack to wait for the redis service to start
        std::thread::sleep(std::time::Duration::from_millis(10000));
        if let Some(status) = spin.try_wait()? {
            anyhow::bail!(
                "Spin exited early with status code {:?}\n{}{}",
                status.code(),
                spin.stdout.output_as_str().unwrap_or("<non-utf8>"),
                spin.stderr.output_as_str().unwrap_or("<non-utf8>")
            );
        }
        Ok(spin)
    }

    /// Make an HTTP request against Spin
    ///
    /// Will fail if Spin has already exited or if the io mode is not HTTP
    pub fn make_http_request(
        &mut self,
        request: Request<'_>,
    ) -> anyhow::Result<reqwest::blocking::Response> {
        let IoMode::Http(port) = self.io_mode else {
            anyhow::bail!("Spin is not running in HTTP mode");
        };
        if let Some(status) = self.try_wait()? {
            anyhow::bail!("Spin exited early with status code {:?}", status.code());
        }
        log::debug!("Connecting to HTTP server on port {port}...");
        let mut outgoing = reqwest::blocking::Request::new(
            request.method,
            reqwest::Url::parse(&format!("http://localhost:{port}"))
                .unwrap()
                .join(request.uri)
                .context("could not construct url for request against Spin")?,
        );
        outgoing
            .headers_mut()
            .extend(request.headers.iter().map(|(k, v)| {
                (
                    reqwest::header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                    reqwest::header::HeaderValue::from_str(v).unwrap(),
                )
            }));
        *outgoing.body_mut() = request.body.map(Into::into);
        let response = reqwest::blocking::Client::new().execute(outgoing)?;
        log::debug!("Awaiting response from server");
        if let Some(status) = self.try_wait()? {
            anyhow::bail!("Spin exited early with status code {:?}", status.code());
        }
        Ok(response)
    }

    pub fn stdout(&mut self) -> &str {
        self.stdout.output_as_str().unwrap_or("<non-utf8>")
    }

    pub fn stderr(&mut self) -> &str {
        self.stderr.output_as_str().unwrap_or("<non-utf8>")
    }

    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.process.try_wait()
    }
}

impl Drop for Spin {
    fn drop(&mut self) {
        kill_process(&mut self.process);
    }
}

impl Runtime for Spin {
    fn error(&mut self) -> anyhow::Result<()> {
        if self.try_wait()?.is_some() {
            anyhow::bail!("Spin exited early: {}", self.stderr());
        }

        Ok(())
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

/// How this Spin instance is communicating with the outside world
enum IoMode {
    Http(u16),
    Redis,
}

/// The mode start Spin up in
pub enum SpinMode {
    Http,
    Redis,
}

/// Uses a track to ge a random unused port
fn get_random_port() -> anyhow::Result<u16> {
    Ok(std::net::TcpListener::bind("localhost:0")?
        .local_addr()?
        .port())
}

/// A request to the spin server
pub struct Request<'a> {
    pub method: reqwest::Method,
    pub uri: &'a str,
    pub headers: &'a [(&'a str, &'a str)],
    pub body: Option<Vec<u8>>,
}

impl<'a> Request<'a> {
    /// Create a new request with no headers or body
    pub fn new(method: reqwest::Method, uri: &'a str) -> Self {
        Self {
            method,
            uri,
            headers: &[],
            body: None,
        }
    }

    /// Create a new request with headers and a body
    pub fn full(
        method: reqwest::Method,
        uri: &'a str,
        headers: &'a [(&'a str, &'a str)],
        body: Option<Vec<u8>>,
    ) -> Self {
        Self {
            method,
            uri,
            headers,
            body,
        }
    }
}
