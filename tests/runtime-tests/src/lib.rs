mod services;

use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::OnceLock,
};

use anyhow::Context;
use services::Services;

/// Configuration for the test suite
pub struct Config {
    /// The path to the Spin binary under test
    pub spin_binary_path: PathBuf,
    /// The path to the tests directory which contains all the runtime test definitions
    pub tests_path: PathBuf,
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
pub fn run_all(config: Config) -> anyhow::Result<()> {
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

        bootstrap_and_run(&test.path(), &config)?;
    }
    Ok(())
}

/// Bootstrap and run a test at a path against the given config
pub fn bootstrap_and_run(test_path: &Path, config: &Config) -> Result<(), anyhow::Error> {
    log::info!("Testing: {}", test_path.display());
    let temp = temp_dir::TempDir::new()
        .context("failed to produce a temporary directory to run the test in")?;
    log::trace!("Temporary directory: {}", temp.path().display());
    let mut services = services::start_services(test_path)?;
    copy_manifest(test_path, &temp, &services)?;
    let spin = Spin::start(&config.spin_binary_path, temp.path(), &mut services)?;
    log::debug!("Spin started on port {}.", spin.port());
    run_test(test_path, spin, config.on_error);
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
                    eprintln!($($arg)*);
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
    let response = match ResponseKind::from_response(response) {
        Ok(r) => r,
        Err(e) => {
            error!(on_error, "failed to parse response from Spin server: {e}")
        }
    };
    let error_file = test_path.join("error.txt");
    match response {
        ResponseKind::Ok if !error_file.exists() => log::info!("Test passed!"),
        ResponseKind::Ok => {
            let expected = match std::fs::read_to_string(&error_file) {
                Ok(e) => e,
                Err(e) => error!(on_error, "failed to read error.txt file: {}", e),
            };
            error!(
                on_error,
                "Test passed but should have failed with error: {expected}"
            )
        }
        ResponseKind::Err(e) if error_file.exists() => {
            let expected = match std::fs::read_to_string(&error_file) {
                Ok(e) => e,
                Err(e) => error!(on_error, "failed to read error.txt file: {}", e),
            };
            if e.contains(&expected) {
                log::info!("Test passed!");
            } else {
                error!(
                    on_error,
                    "Test errored but not in the expected way.\n\texpected: {}\n\tgot: {}\n\nSpin stderr:\n{}",
                    expected,
                    e,
                    spin.stderr.output_as_str().unwrap_or("<non-utf8>")

                )
            }
        }
        // An empty error message may indicate that the component panicked
        ResponseKind::Err(e) if e.is_empty() => {
            let e = spin
                .stderr
                .output_as_str()
                .unwrap_or("Spin server did not return body and did not write to stderr");
            error!(on_error, "Test '{}' errored: {e}", test_path.display());
        }
        ResponseKind::Err(e) => {
            error!(
                on_error,
                "Test '{}' errored: {e}\nSpin stderr: {}",
                test_path.display(),
                spin.stderr.output_as_str().unwrap_or("<non-utf8>")
            );
        }
    }
    if let OnTestError::Log = on_error {
        println!("'{}' passed", test_path.display())
    }
}

static TEMPLATE: OnceLock<regex::Regex> = OnceLock::new();
/// Copies the test dir's manifest file into the temporary directory
///
/// Replaces template variables in the manifest file with components from the components path.
fn copy_manifest(
    test_dir: &Path,
    temp: &temp_dir::TempDir,
    services: &Services,
) -> anyhow::Result<()> {
    let manifest_path = test_dir.join("spin.toml");
    let mut manifest = std::fs::read_to_string(manifest_path).with_context(|| {
        format!(
            "no spin.toml manifest found in test directory {}",
            test_dir.display()
        )
    })?;
    let regex = TEMPLATE.get_or_init(|| regex::Regex::new(r"%\{(.*?)\}").unwrap());
    while let Some(captures) = regex.captures(&manifest) {
        let (Some(full), Some(capture)) = (captures.get(0), captures.get(1)) else {
            continue;
        };
        let template = capture.as_str();
        let (template_key, template_value) = template.split_once('=').with_context(|| {
            format!("invalid template '{template}'(template should be in the form $KEY=$VALUE)")
        })?;
        let replacement = match template_key.trim() {
            "source" => {
                let path = std::path::PathBuf::from(
                    test_components::path(template_value)
                        .with_context(|| format!("no such component '{template_value}'"))?,
                );
                let wasm_name = path.file_name().unwrap().to_str().unwrap();
                std::fs::copy(&path, temp.path().join(wasm_name)).with_context(|| {
                    format!(
                        "failed to copy wasm file '{}' to temporary directory",
                        path.display()
                    )
                })?;
                wasm_name.to_owned()
            }
            "port" => {
                let guest_port = template_value
                    .parse()
                    .with_context(|| format!("failed to parse '{template_value}' as port"))?;
                let port = services
                    .get_port(guest_port)?
                    .with_context(|| format!("no port {guest_port} exposed by any service"))?;
                port.to_string()
            }
            _ => {
                anyhow::bail!("unknown template key: {template_key}");
            }
        };
        manifest.replace_range(full.range(), &replacement);
    }
    std::fs::write(temp.path().join("spin.toml"), manifest)
        .context("failed to copy spin.toml manifest to temporary directory")?;
    Ok(())
}

#[derive(Debug)]
enum ResponseKind {
    Ok,
    Err(String),
}

impl ResponseKind {
    fn from_response(response: reqwest::blocking::Response) -> anyhow::Result<Self> {
        if response.status() == 200 {
            return Ok(Self::Ok);
        }
        if response.status() != 500 {
            anyhow::bail!("Response was neither 200 nor 500")
        }

        Ok(Self::Err(response.text()?))
    }
}

fn make_http_request(spin: &mut Spin) -> Result<reqwest::blocking::Response, anyhow::Error> {
    if let Some(status) = spin.try_wait()? {
        anyhow::bail!("Spin exited early with status code {:?}", status.code());
    }
    log::debug!("Connecting to HTTP server on port {}...", spin.port());
    let response = reqwest::blocking::get(format!("http://127.0.0.1:{}", spin.port()))?;
    log::debug!("Awaiting response from server");
    if let Some(status) = spin.try_wait()? {
        anyhow::bail!("Spin exited early with status code {:?}", status.code());
    }
    Ok(response)
}

struct Spin {
    process: std::process::Child,
    #[allow(dead_code)]
    stdout: OutputStream,
    stderr: OutputStream,
    port: u16,
}

impl Spin {
    fn start(
        spin_binary_path: &Path,
        current_dir: &Path,
        services: &mut services::Services,
    ) -> Result<Self, anyhow::Error> {
        let port = get_random_port()?;
        let mut child = Command::new(spin_binary_path)
            .arg("up")
            .current_dir(current_dir)
            .args(["--listen", &format!("127.0.0.1:{port}")])
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
            port,
        };
        for _ in 0..80 {
            services.error()?;
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
            "`spin up` did not start server or error after 20 seconds. stderr:\n\t{}",
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
