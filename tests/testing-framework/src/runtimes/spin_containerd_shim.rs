use std::{
    path::Path,
    process::{Command, Stdio},
};

use crate::{
    http::{Request, Response},
    io::OutputStream,
    runtimes::spin_cli::{kill_process, IoMode, SpinConfig},
    Runtime, TestEnvironment,
};

use anyhow::Context as _;

pub struct SpinShim {
    process: std::process::Child,
    #[allow(dead_code)]
    stdout: OutputStream,
    stderr: OutputStream,
    io_mode: IoMode,
}

impl SpinShim {
    pub fn regisry_push<R>(
        spin_binary_path: &Path,
        image: &str,
        env: &mut TestEnvironment<R>,
    ) -> anyhow::Result<()> {
        // TODO: consider enabling configuring a port
        Command::new(spin_binary_path)
            .args(["registry", "push"])
            .arg(image)
            .current_dir(env.path())
            .output()
            .context("failed to push spin app to registry with 'spin'")?;
        // TODO: assess output
        Ok(())
    }

    pub fn image_pull(ctr_binary_path: &Path, image: &str) -> anyhow::Result<()> {
        // TODO: consider enabling configuring a port
        Command::new(ctr_binary_path)
            .args(["image", "pull"])
            .arg(image)
            .output()
            .context("failed to pull spin app with 'ctr'")?;
        // TODO: assess output
        Ok(())
    }

    /// Start the Spin app using `ctr run`
    /// Equivalent of `sudo ctr run --rm --net-host --runtime io.containerd.spin.v2 ttl.sh/myapp:48h ctr-run-id bogus-arg` for image `ttl.sh/myapp:48h` and run id `ctr-run-id`
    pub fn start<R>(
        ctr_binary_path: &Path,
        env: &mut TestEnvironment<R>,
        image: &str,
        ctr_run_id: &str,
    ) -> anyhow::Result<Self> {
        let port = 80;
        let mut ctr_cmd = std::process::Command::new(ctr_binary_path);
        let child = ctr_cmd
            .arg("run")
            .args(["--rm", "--net-host", "--runtime", "io.containerd.spin.v2"])
            .arg(image)
            .arg(ctr_run_id)
            .arg("bogus-arg")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in env.env_vars() {
            child.env(key, value);
        }
        let mut child = child.spawn()?;
        let stdout = OutputStream::new(child.stdout.take().unwrap());
        let stderr = OutputStream::new(child.stderr.take().unwrap());
        log::debug!("Awaiting shim binary to start up on port {port}...");
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
                    log::debug!("Spin shim started on port {port}.");
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
                    "Shim exited early with status code {:?}\n{}{}",
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
            "`ctr run` did not start server or error after two minutes. stderr:\n\t{}",
            spin.stderr.output_as_str().unwrap_or("<non-utf8>")
        )
    }

    /// Make an HTTP request against Spin
    ///
    /// Will fail if Spin has already exited or if the io mode is not HTTP
    pub fn make_http_request<B: Into<reqwest::Body>>(
        &mut self,
        request: Request<'_, B>,
    ) -> anyhow::Result<Response> {
        let IoMode::Http(port) = self.io_mode else {
            anyhow::bail!("Spin shim is not running in HTTP mode");
        };
        if let Some(status) = self.try_wait()? {
            anyhow::bail!(
                "make_http_request - shim exited early with status code {:?}",
                status.code()
            );
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
            Result::<_, anyhow::Error>::Ok(Response::full(
                response.status().as_u16(),
                response
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
            ))
        })?;
        log::debug!("Awaiting response from server");
        if let Some(status) = self.try_wait()? {
            anyhow::bail!("Spin exited early with status code {:?}", status.code());
        }
        println!("Response: {}", response.status());
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

impl Drop for SpinShim {
    fn drop(&mut self) {
        kill_process(&mut self.process);
    }
}

impl Runtime for SpinShim {
    type Config = SpinConfig;

    fn error(&mut self) -> anyhow::Result<()> {
        if !matches!(self.io_mode, IoMode::None) && self.try_wait()?.is_some() {
            anyhow::bail!("Containerd shim spin exited early: {}", self.stderr());
        }

        Ok(())
    }
}
