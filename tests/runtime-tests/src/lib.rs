use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::Context;

/// Configuration for the test suite
pub struct Config {
    /// The path to the Spin binary under test
    pub spin_binary_path: PathBuf,
    /// The path to the tests directory which contains all the runtime test definitions
    pub tests_path: PathBuf,
    /// The path to the shared repository of WebAssembly components that the test suite uses
    pub components_path: PathBuf,
    /// What to do when an individual test fails
    pub on_error: OnTestError,
}

#[derive(Clone, Copy)]
/// What to do on a test error
pub enum OnTestError {
    Panic,
    Log,
}

/// Run the runtime tests suite.
///
/// Error represents an error in bootstrapping the tests. What happens on individual test failures
/// is controlled by `config`.
pub fn run(config: Config) -> anyhow::Result<()> {
    for test in std::fs::read_dir(&config.tests_path).with_context(|| {
        format!(
            "failed to read test directory '{}'",
            config.tests_path.display()
        )
    })? {
        let test = test.context("I/O error reading entry from test directory")?;
        if !test.file_type()?.is_dir() {
            log::debug!(
                "Ignoring non-sub-directory in test directory: {}",
                test.path().display()
            );
            continue;
        }

        log::info!("Testing: {}", test.path().display());
        let temp = temp_dir::TempDir::new()
            .context("failed to produce a temporary directory to run the test in")?;
        log::trace!("Temporary directory: {}", temp.path().display());
        copy_manifest(&test.path(), &config.components_path, &temp)?;
        let spin = Spin::start(&config.spin_binary_path, temp.path())?;
        log::debug!("Spin started on port {}.", spin.port());
        run_test(test.path().as_path(), spin, config.on_error)
    }
    Ok(())
}

/// Run an individual test
fn run_test(test_path: &Path, mut spin: Spin, on_error: OnTestError) {
    // macro which will look at `on_error` and do the right thing
    macro_rules! error {
        ($on_error:expr, $($arg:tt)*) => {
            match $on_error {
                OnTestError::Panic => panic!($($arg)*),
                OnTestError::Log => {
                    println!($($arg)*);
                    return;
                }
            }
        };
    }

    let response = match make_http_request(&mut spin) {
        Ok(r) => r,
        Err(e) => {
            error!(
                on_error,
                "Test failed trying to connect to http server: {e}"
            );
        }
    };
    let response: Response = match response.parse() {
        Ok(r) => r,
        Err(e) => {
            error!(on_error, "failed to parse response from Spin server: {e}")
        }
    };
    let error_file = test_path.join("error.txt");
    match response {
        Response::Ok if !error_file.exists() => log::info!("Test passed!"),
        Response::Ok => {
            let expected = match std::fs::read_to_string(&error_file) {
                Ok(e) => e,
                Err(e) => error!(on_error, "failed to read error.txt file: {}", e),
            };
            error!(
                on_error,
                "Test passed but should have failed with error: {expected}"
            )
        }
        Response::Err(e) if error_file.exists() => {
            let expected = match std::fs::read_to_string(&error_file) {
                Ok(e) => e,
                Err(e) => error!(on_error, "failed to read error.txt file: {}", e),
            };
            if e.contains(&expected) {
                log::info!("Test passed!");
            } else {
                error!(
                    on_error,
                    "Test errored but not in the expected way.\n\texpected: {}\n\tgot: {}",
                    expected,
                    e
                )
            }
        }
        Response::Err(e) => {
            error!(on_error, "Test errored: {e}");
        }
        Response::ErrNoBody => {
            let stderr = spin.stderr.output_as_str().unwrap_or("<non-utf8>");
            error!(on_error, "Spin did not return error body (most likely due to a test panicking). stderr:\n{stderr}");
        }
    }
}

/// Copies the test dir's manifest file into the temporary directory
///
/// Replaces template variables in the manifest file with components from the components path.
fn copy_manifest(
    test_dir: &Path,
    components_path: &Path,
    temp: &temp_dir::TempDir,
) -> anyhow::Result<()> {
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
            let path = component_path(components_path, name);
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
            s.as_bytes()[s.len() - 4..] == b"\r\n\r\n"[..],
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
    log::debug!("Connecting to HTTP server on port {}...", spin.port());
    let mut connection = std::net::TcpStream::connect(format!("127.0.0.1:{}", spin.port()))
        .context("could not connect to Spin web server")?;
    connection.write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1:3000\r\n\r\n")?;
    log::debug!("Awaiting response from server");
    let mut start = 0;
    let mut output = vec![0; 1024];
    for _ in 0..20 {
        let n = connection.read(&mut output[start..])?;
        start += n;
        let header_end = output.windows(4).position(|c| c == b"\r\n\r\n");
        if let Some(header_end) = header_end {
            let until_headers = String::from_utf8(output[..header_end].to_vec())
                .context("spin HTTP response headers contained non-utf8 bytes")?;
            if let Some(s) = until_headers.find("content-length: ") {
                if std::str::from_utf8(&until_headers.as_bytes()[s + 16..])
                    .unwrap()
                    .starts_with('0')
                {
                    let response_with_no_body =
                        String::from_utf8(output[..header_end + 4].to_vec()).unwrap();
                    return Ok(response_with_no_body);
                }
            }
            if output[header_end + 4..]
                .windows(4)
                .any(|c| c == b"\r\n\r\n")
            {
                let response = String::from_utf8(output[..start].to_vec())
                    .context("spin HTTP response contained non-utf8 bytes")?;
                return Ok(response);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
        if let Some(status) = spin.try_wait()? {
            anyhow::bail!("Spin exited early with status code {:?}", status.code());
        }
    }
    anyhow::bail!("did not return http response after 5 seconds")
}

struct Spin {
    process: std::process::Child,
    #[allow(dead_code)]
    stdout: OutputStream,
    stderr: OutputStream,
    port: u16,
}

impl Spin {
    fn start(spin_binary_path: &Path, current_dir: &Path) -> Result<Self, anyhow::Error> {
        let port = get_random_port()?;
        let mut child = std::process::Command::new(spin_binary_path)
            .arg("up")
            .current_dir(current_dir)
            .args(["--listen", &format!("127.0.0.1:{port}")])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        let stdout = OutputStream::new(child.stdout.take().unwrap());
        let stderr = OutputStream::new(child.stderr.take().unwrap());
        log::debug!("Awaiting spin binary to start up on port {port}...");
        let mut spin = Self {
            process: child,
            stdout,
            stderr,
            port,
        };
        for _ in 0..20 {
            match std::net::TcpStream::connect(format!("127.0.0.1:{port}")) {
                Ok(_) => return Ok(spin),
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

            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        anyhow::bail!(
            "`spin up` did not start server or error after 5 seconds. stderr:\n\t{}",
            spin.stderr.output_as_str().unwrap_or("<non-utf8>")
        )
    }

    /// The port Spin is running on
    fn port(&self) -> u16 {
        self.port
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

fn component_path(test_components_path: &Path, name: &str) -> PathBuf {
    test_components_path.join(name).join("component.wasm")
}

/// Uses a track to ge a random unused port
fn get_random_port() -> anyhow::Result<u16> {
    Ok(std::net::TcpListener::bind("localhost:0")?
        .local_addr()?
        .port())
}

/// Helper for reading from a child process stream in a non-blocking way
struct OutputStream {
    rx: std::sync::mpsc::Receiver<Result<Vec<u8>, std::io::Error>>,
    buffer: Vec<u8>,
}

impl OutputStream {
    fn new<R: Read + Send + 'static>(mut stream: R) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut buffer = vec![0; 1024];
            loop {
                if tx
                    .send(stream.read(&mut buffer).map(|n| buffer[..n].to_vec()))
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            rx,
            buffer: Vec::new(),
        }
    }

    /// Get the output of the stream so far
    fn output(&mut self) -> &[u8] {
        while let Ok(Ok(s)) = self.rx.try_recv() {
            self.buffer.extend(s);
        }
        &self.buffer
    }

    /// Get the output of the stream so far
    fn output_as_str(&mut self) -> Option<&str> {
        std::str::from_utf8(self.output()).ok()
    }
}
