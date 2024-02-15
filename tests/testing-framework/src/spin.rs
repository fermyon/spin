use anyhow::Context;

use crate::{io::OutputStream, Runtime, TestEnvironment};
use std::{
    collections::HashMap,
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
    pub fn start<R>(
        spin_binary_path: &Path,
        env: &TestEnvironment<R>,
        spin_up_args: Vec<impl AsRef<std::ffi::OsStr>>,
        mode: SpinMode,
    ) -> anyhow::Result<Self> {
        match mode {
            SpinMode::Http => Self::start_http(spin_binary_path, env, spin_up_args),
            SpinMode::Redis => Self::start_redis(spin_binary_path, env, spin_up_args),
            SpinMode::None => Self::attempt_start(spin_binary_path, env, spin_up_args),
        }
    }

    /// Start Spin in `current_dir` using the binary at `spin_binary_path`
    pub fn start_http<R>(
        spin_binary_path: &Path,
        env: &TestEnvironment<R>,
        spin_up_args: Vec<impl AsRef<std::ffi::OsStr>>,
    ) -> anyhow::Result<Self> {
        let port = get_random_port()?;
        let mut child = Command::new(spin_binary_path)
            .arg("up")
            .current_dir(env.path())
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

    pub fn start_redis<R>(
        spin_binary_path: &Path,
        env: &TestEnvironment<R>,
        spin_up_args: Vec<impl AsRef<std::ffi::OsStr>>,
    ) -> anyhow::Result<Self> {
        let mut child = Command::new(spin_binary_path)
            .arg("up")
            .current_dir(env.path())
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

    fn attempt_start<R>(
        spin_binary_path: &Path,
        env: &TestEnvironment<R>,
        spin_up_args: Vec<impl AsRef<std::ffi::OsStr>>,
    ) -> anyhow::Result<Self> {
        let mut child = Command::new(spin_binary_path)
            .arg("up")
            .current_dir(env.path())
            .args(spin_up_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdout = OutputStream::new(child.stdout.take().unwrap());
        let stderr = OutputStream::new(child.stderr.take().unwrap());
        child.wait()?;
        Ok(Self {
            process: child,
            stdout,
            stderr,
            io_mode: IoMode::None,
        })
    }

    /// Make an HTTP request against Spin
    ///
    /// Will fail if Spin has already exited or if the io mode is not HTTP
    pub fn make_http_request<B: Into<reqwest::Body>>(
        &mut self,
        request: Request<'_, B>,
    ) -> anyhow::Result<Response> {
        let IoMode::Http(port) = self.io_mode else {
            anyhow::bail!("Spin is not running in HTTP mode");
        };
        if let Some(status) = self.try_wait()? {
            anyhow::bail!("Spin exited early with status code {:?}", status.code());
        }
        log::debug!("Connecting to HTTP server on port {port}...");
        let mut outgoing = reqwest::Request::new(
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
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let response = rt.block_on(async {
            let mut retries = 0;
            let mut response = loop {
                let Some(request) = outgoing.try_clone() else {
                    break reqwest::Client::new().execute(outgoing).await;
                };
                let response = reqwest::Client::new().execute(request).await;
                if response.is_err() && retries < 5 {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    retries += 1;
                } else {
                    break response;
                }
            }?;
            let mut chunks = Vec::new();
            while let Some(chunk) = response.chunk().await? {
                chunks.push(chunk.to_vec());
            }
            Result::<_, anyhow::Error>::Ok(Response {
                status: response.status().as_u16(),
                headers: response
                    .headers()
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            k.as_str().to_owned(),
                            v.to_str().unwrap_or("<non-utf8>").to_owned(),
                        )
                    })
                    .collect(),
                chunks,
            })
        })?;
        log::debug!("Awaiting response from server");
        if let Some(status) = self.try_wait()? {
            anyhow::bail!("Spin exited early with status code {:?}", status.code());
        }
        Ok(response)
    }

    /// Get the HTTP URL of the Spin server if running in http mode
    pub fn http_url(&self) -> Option<String> {
        match self.io_mode {
            IoMode::Http(port) => Some(format!("http://localhost:{}", port)),
            _ => None,
        }
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
    type Config = SpinConfig;

    fn error(&mut self) -> anyhow::Result<()> {
        if !matches!(self.io_mode, IoMode::None) && self.try_wait()?.is_some() {
            anyhow::bail!("Spin exited early: {}", self.stderr());
        }

        Ok(())
    }
}

pub struct SpinConfig {
    pub binary_path: std::path::PathBuf,
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
    /// An http server is running on this port
    Http(u16),
    /// Spin is running in redis mode
    Redis,
    /// Spin may or may not be running
    None,
}

/// The mode start Spin up in
pub enum SpinMode {
    /// Expect an http listener to start
    Http,
    /// Expect a redis listener to start
    Redis,
    /// Don't expect spin to start
    None,
}

/// Uses a track to ge a random unused port
fn get_random_port() -> anyhow::Result<u16> {
    Ok(std::net::TcpListener::bind("localhost:0")?
        .local_addr()?
        .port())
}

/// A request to the spin server
pub struct Request<'a, B> {
    pub method: reqwest::Method,
    pub uri: &'a str,
    pub headers: &'a [(&'a str, &'a str)],
    pub body: Option<B>,
}

impl<'a, 'b> Request<'a, &'b [u8]> {
    /// Create a new request with no headers or body
    pub fn new(method: reqwest::Method, uri: &'a str) -> Self {
        Self {
            method,
            uri,
            headers: &[],
            body: None,
        }
    }
}

impl<'a, B> Request<'a, B> {
    /// Create a new request with headers and a body
    pub fn full(
        method: reqwest::Method,
        uri: &'a str,
        headers: &'a [(&'a str, &'a str)],
        body: Option<B>,
    ) -> Self {
        Self {
            method,
            uri,
            headers,
            body,
        }
    }
}

pub struct Response {
    status: u16,
    headers: HashMap<String, String>,
    chunks: Vec<Vec<u8>>,
}

impl Response {
    /// A response with no headers or body
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: Default::default(),
            chunks: Default::default(),
        }
    }

    /// A response with headers and a body
    pub fn new_with_body(status: u16, chunks: impl IntoChunks) -> Self {
        Self {
            status,
            headers: Default::default(),
            chunks: chunks.into_chunks(),
        }
    }

    /// A response with headers and a body
    pub fn full(status: u16, headers: HashMap<String, String>, chunks: impl IntoChunks) -> Self {
        Self {
            status,
            headers,
            chunks: chunks.into_chunks(),
        }
    }

    /// The status code of the response
    pub fn status(&self) -> u16 {
        self.status
    }

    /// The headers of the response
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    /// The body of the response
    pub fn body(&self) -> Vec<u8> {
        self.chunks.iter().flatten().copied().collect()
    }

    /// The body of the response as chunks of bytes
    ///
    /// If the response is not stream this will be a single chunk equal to the body
    pub fn chunks(&self) -> &[Vec<u8>] {
        &self.chunks
    }

    /// The body of the response as a string
    pub fn text(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.body())
    }
}

pub trait IntoChunks {
    fn into_chunks(self) -> Vec<Vec<u8>>;
}

impl IntoChunks for Vec<Vec<u8>> {
    fn into_chunks(self) -> Vec<Vec<u8>> {
        self
    }
}

impl IntoChunks for Vec<u8> {
    fn into_chunks(self) -> Vec<Vec<u8>> {
        vec![self]
    }
}

impl IntoChunks for String {
    fn into_chunks(self) -> Vec<Vec<u8>> {
        vec![self.into_bytes()]
    }
}

impl IntoChunks for &str {
    fn into_chunks(self) -> Vec<Vec<u8>> {
        vec![self.as_bytes().into()]
    }
}
