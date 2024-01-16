#[cfg(test)]
mod integration_tests {
    use anyhow::{anyhow, Context, Error, Result};
    use futures::{channel::oneshot, future, stream, FutureExt};
    use http_body_util::BodyExt;
    use hyper::{body::Bytes, server::conn::http1, service::service_fn, Method, StatusCode};
    use hyper_util::rt::tokio::TokioIo;
    use reqwest::Client;
    use sha2::{Digest, Sha256};
    use spin_http::body;
    use spin_manifest::schema::v2;
    use std::{
        collections::HashMap,
        ffi::OsStr,
        iter,
        net::{Ipv4Addr, SocketAddrV4, TcpListener},
        path::{Path, PathBuf},
        process::{self, Child, Command, Output},
        sync::{Arc, Mutex},
        time::Duration,
    };
    use tempfile::tempdir;
    use tokio::{net::TcpStream, task, time::sleep};
    use tracing::log;

    const TIMER_TRIGGER_INTEGRATION_TEST: &str = "examples/spin-timer/app-example";
    const TIMER_TRIGGER_DIRECTORY: &str = "examples/spin-timer";

    const DEFAULT_MANIFEST_LOCATION: &str = "spin.toml";

    fn spin_binary() -> String {
        env!("CARGO_BIN_EXE_spin").into()
    }

    #[tokio::test]
    async fn test_timer_trigger() -> Result<()> {
        use std::fs;

        let trigger_dir = Path::new(TIMER_TRIGGER_DIRECTORY);

        // Conventionally, we would do all Cargo builds of test code in build.rs, but this one can take a lot
        // longer than the tiny tests we normally build there (and it's pointless if the user just wants to build
        // Spin without running any tests) so we do it here instead.  Subsequent builds after the first one should
        // be very fast.
        assert!(Command::new("cargo")
            .arg("build")
            .arg("--release")
            .arg("--target-dir")
            .arg(trigger_dir.join("target"))
            .arg("--manifest-path")
            .arg(trigger_dir.join("Cargo.toml"))
            .status()?
            .success());

        // Create a test plugin store so we don't modify the user's real one.
        let plugin_store_dir = Path::new(concat!(env!("OUT_DIR"), "/plugin-store"));
        let plugins_dir = plugin_store_dir.join("spin/plugins");

        let plugin_dir = plugins_dir.join("trigger-timer");
        fs::create_dir_all(&plugin_dir)?;
        fs::copy(
            trigger_dir.join("target/release/trigger-timer"),
            plugin_dir.join("trigger-timer"),
        )
        .context("could not copy plugin binary into plugin directory")?;

        let manifests_dir = plugins_dir.join("manifests");
        fs::create_dir_all(&manifests_dir)?;
        // Note that the hash and path in the manifest aren't accurate, but they won't be used anyway for this
        // test. We just need something that parses without throwing errors here.
        fs::copy(
            Path::new(TIMER_TRIGGER_DIRECTORY).join("trigger-timer.json"),
            manifests_dir.join("trigger-timer.json"),
        )
        .context("could not copy plugin manifest into manifests directory")?;

        let out = Command::new(get_process(&spin_binary()))
            .args([
                "up",
                "--file",
                &format!("{TIMER_TRIGGER_INTEGRATION_TEST}/{DEFAULT_MANIFEST_LOCATION}"),
                "--test",
            ])
            .env("TEST_PLUGINS_DIRECTORY", plugin_store_dir)
            .output()?;
        assert!(
            out.status.success(),
            "Running `spin up` returned error: {}",
            String::from_utf8_lossy(&out.stderr)
        );

        Ok(())
    }

    fn integration_test_with_args(
        test_path: impl Into<PathBuf>,
        test: impl FnOnce(
                &mut testing_framework::TestEnvironment<testing_framework::Spin>,
            ) -> testing_framework::TestResult<anyhow::Error>
            + 'static,
        spin_up_args: Vec<String>,
        services: testing_framework::ServicesConfig,
    ) -> anyhow::Result<()> {
        let test_path = test_path.into();
        let spin = testing_framework::TestEnvironmentConfig::spin(
            spin_binary().into(),
            spin_up_args,
            move |env| {
                for file in std::fs::read_dir(test_path)? {
                    let file = file?;
                    let path = file.path();
                    if path.is_dir() {
                        env.copy_into(&path, "")?;
                    } else {
                        let mut template = testing_framework::ManifestTemplate::from_file(&path)?;
                        template.substitute(env)?;
                        env.write_file(path.file_name().unwrap(), template.contents())?;
                    }
                }

                Ok(())
            },
            services,
            testing_framework::SpinMode::Http,
        );
        let mut env = testing_framework::TestEnvironment::up(spin)?;
        test(&mut env)?;
        Ok(())
    }

    fn assert_spin_status(
        spin: &mut testing_framework::Spin,
        uri: &str,
        status: u16,
    ) -> testing_framework::TestResult<anyhow::Error> {
        let r =
            spin.make_http_request(testing_framework::Request::new(reqwest::Method::GET, uri))?;
        if r.status() != status {
            return Err(testing_framework::TestError::Failure(anyhow!(
                "Expected status {} for {} but got {}\nbody: {}\nstderr: {}",
                status,
                uri,
                r.status(),
                r.text().unwrap_or_else(|_| String::from("<non-utf8>")),
                spin.stderr()
            )));
        }
        Ok(())
    }

    /// Controller for running Spin.
    pub struct SpinTestController {
        pub url: String,
        spin_handle: Child,
    }

    impl SpinTestController {
        pub async fn with_manifest(
            manifest_path: &str,
            spin_args: &[&str],
            spin_app_env: &[&str],
        ) -> Result<SpinTestController> {
            // start Spin using the given application manifest and wait for the HTTP server to be available.
            let url = format!("127.0.0.1:{}", get_random_port()?);
            let mut args = vec!["up", "--file", manifest_path, "--listen", &url];
            args.extend(spin_args);
            for v in spin_app_env {
                args.push("--env");
                args.push(v);
            }

            let mut spin_handle = Command::new(get_process(&spin_binary()))
                .args(args)
                .env(
                    "RUST_LOG",
                    "spin=trace,spin_loader=trace,spin_core=trace,spin_http=trace",
                )
                .spawn()
                .with_context(|| "executing Spin")?;

            // ensure the server is accepting requests before continuing.
            wait_tcp(&url, &mut spin_handle, &spin_binary()).await?;

            Ok(SpinTestController { url, spin_handle })
        }
    }

    impl Drop for SpinTestController {
        fn drop(&mut self) {
            #[cfg(windows)]
            let _ = self.spin_handle.kill();
            #[cfg(not(windows))]
            {
                let pid = nix::unistd::Pid::from_raw(self.spin_handle.id() as i32);
                let _ = nix::sys::signal::kill(pid, nix::sys::signal::SIGTERM);
            }
        }
    }

    fn run<S: Into<String> + AsRef<OsStr>>(
        args: Vec<S>,
        dir: Option<S>,
        envs: Option<HashMap<&str, &str>>,
    ) -> Result<Output> {
        let mut cmd = Command::new(get_os_process());
        cmd.stdout(process::Stdio::piped());
        cmd.stderr(process::Stdio::piped());

        if let Some(dir) = dir {
            cmd.current_dir(dir.into());
        };

        cmd.arg("-c");
        cmd.arg(
            args.into_iter()
                .map(Into::into)
                .collect::<Vec<String>>()
                .join(" "),
        );

        cmd.env("RUST_LOG", "spin_cli=warn");
        if let Some(envs) = envs {
            for (k, v) in envs {
                cmd.env(k, v);
            }
        }

        let output = cmd.output()?;
        println!("STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
        println!("STDERR:\n{}", String::from_utf8_lossy(&output.stderr));

        let code = output.status.code().expect("should have status code");
        if code != 0 {
            panic!("command `{:?}` exited with code {}", cmd, code);
        }

        Ok(output)
    }

    fn get_process(binary: &str) -> String {
        if cfg!(target_os = "windows") {
            format!("{}.exe", binary)
        } else {
            binary.to_owned()
        }
    }

    fn get_os_process() -> String {
        if cfg!(target_os = "windows") {
            String::from("powershell.exe")
        } else {
            String::from("bash")
        }
    }

    fn get_random_port() -> Result<u16> {
        Ok(
            TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))?
                .local_addr()?
                .port(),
        )
    }

    async fn wait_tcp(url: &str, process: &mut Child, target: &str) -> Result<()> {
        let mut wait_count = 0;
        loop {
            if wait_count >= 240 {
                panic!(
                    "Ran out of retries waiting for {} to start on URL {}",
                    target, url
                );
            }

            if let Ok(Some(_)) = process.try_wait() {
                panic!(
                    "Process exited before starting to serve {} to start on URL {}",
                    target, url
                );
            }

            match TcpStream::connect(&url).await {
                Ok(_) => break,
                Err(e) => {
                    println!("connect {} error {}, retry {}", &url, e, wait_count);
                    wait_count += 1;
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }

        Ok(())
    }

    /// Builds app in `dir` and verifies the build succeeded. Expects manifest
    /// in `spin.toml` inside `dir`.
    async fn do_test_build_command(dir: impl AsRef<Path>) -> Result<()> {
        let dir = dir.as_ref();
        let manifest_file = dir.join("spin.toml");
        let manifest = spin_manifest::manifest_from_file(&manifest_file)?;

        let sources = manifest
            .components
            .iter()
            .map(|(id, component)| {
                let v2::ComponentSource::Local(file) = &component.source else {
                    panic!(
                        "{}.{}: source is not a file reference",
                        manifest.application.name, id
                    )
                };
                (id, dir.join(file))
            })
            .collect::<HashMap<_, _>>();

        // Delete build output so that later it can be assumed: if the output
        // exists, it is because `spin build` succeeded.
        for source in sources.values() {
            if source.exists() {
                std::fs::remove_file(source)?
            }
        }

        run(
            vec![
                spin_binary().as_str(),
                "build",
                "--file",
                manifest_file.to_str().unwrap(),
            ],
            None,
            None,
        )?;

        let mut missing_sources_count = 0;
        for (component_id, source) in sources.iter() {
            if source.exists() {
                std::fs::remove_file(source)?;
            } else {
                missing_sources_count += 1;
                println!(
                    "{}.{} source file was not generated by build",
                    manifest.application.name, component_id
                );
            }
        }
        assert_eq!(missing_sources_count, 0);

        Ok(())
    }

    #[test]
    fn spin_up_gives_help_on_new_app() -> Result<()> {
        let temp_dir = tempdir()?;
        let dir = temp_dir.path();
        let manifest_file = dir.join("spin.toml");

        // We still don't see full help if there are no components.
        let toml_text = r#"spin_version = "1"
name = "unbuilt"
trigger = { type = "http" }
version = "0.1.0"
[[component]]
id = "unbuilt"
source = "fake.wasm"
[component.trigger]
route = "/..."
"#;

        std::fs::write(&manifest_file, toml_text)?;
        std::fs::write(dir.join("fake.wasm"), "")?;

        let binary = spin_binary();
        let up_help_args = vec![
            &binary,
            "up",
            "--file",
            manifest_file.to_str().unwrap(),
            "--help",
        ];

        let output = run(up_help_args, None, None)?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("--quiet"));
        assert!(stdout.contains("--listen"));

        Ok(())
    }

    // TODO: Test on Windows
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_spin_plugin_install_command() -> Result<()> {
        // Create a temporary directory for plugin source and manifests
        let temp_dir = tempdir()?;
        let dir = temp_dir.path();
        let installed_plugins_dir = dir.join("tmp");

        // Ensure that spin installs the plugins into the temporary directory
        let mut env_map: HashMap<&str, &str> = HashMap::new();
        env_map.insert(
            "TEST_PLUGINS_DIRECTORY",
            installed_plugins_dir.to_str().unwrap(),
        );

        let path_to_test_dir = std::env::current_dir()?;
        let file_url = format!(
            "file:{}/tests/plugin/example.tar.gz",
            path_to_test_dir.to_str().unwrap()
        );
        let mut plugin_manifest_json = serde_json::json!(
        {
            "name": "example",
            "description": "A description of the plugin.",
            "homepage": "www.example.com",
            "version": "0.2.0",
            "spinCompatibility": ">=0.5",
            "license": "MIT",
            "packages": [
                {
                    "os": "linux",
                    "arch": "amd64",
                    "url": file_url,
                    "sha256": "f7a5a8c16a94fe934007f777a1bf532ef7e42b02133e31abf7523177b220a1ce"
                },
                {
                    "os": "macos",
                    "arch": "aarch64",
                    "url": file_url,
                    "sha256": "f7a5a8c16a94fe934007f777a1bf532ef7e42b02133e31abf7523177b220a1ce"
                },
                {
                    "os": "macos",
                    "arch": "amd64",
                    "url": file_url,
                    "sha256": "f7a5a8c16a94fe934007f777a1bf532ef7e42b02133e31abf7523177b220a1ce"
                }
            ]
        });
        let manifest_file_path = dir.join("example-plugin-manifest.json");
        std::fs::write(
            &manifest_file_path,
            serde_json::to_string(&plugin_manifest_json).unwrap(),
        )?;

        // Install plugin
        let binary = spin_binary();
        let install_args = vec![
            &binary,
            "plugins",
            "install",
            "--file",
            manifest_file_path.to_str().unwrap(),
            "--yes",
        ];
        run(install_args, None, Some(env_map.clone()))?;

        // Execute example plugin which writes "This is an example Spin plugin!" to a specified file
        let binary = spin_binary();
        let execute_args = vec![&binary, "example"];
        let output = run(execute_args, None, Some(env_map.clone()))?;

        // Verify plugin successfully wrote to output file
        assert!(std::str::from_utf8(&output.stdout)?
            .trim()
            .contains("This is an example Spin plugin!"));

        // Upgrade plugin to newer version
        *plugin_manifest_json.get_mut("version").unwrap() = serde_json::json!("0.2.1");
        std::fs::write(
            dir.join("example-plugin-manifest.json"),
            serde_json::to_string(&plugin_manifest_json).unwrap(),
        )?;
        let binary = spin_binary();
        let upgrade_args = vec![
            &binary,
            "plugins",
            "upgrade",
            "example",
            "--file",
            manifest_file_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Cannot convert PathBuf to str"))?,
            "--yes",
        ];
        run(upgrade_args, None, Some(env_map))?;

        // Check plugin version
        let installed_manifest = installed_plugins_dir
            .join("spin")
            .join("plugins")
            .join("manifests")
            .join("example.json");
        let manifest = std::fs::read_to_string(installed_manifest)?;
        assert!(manifest.contains("0.2.1"));

        // Uninstall plugin
        let binary = spin_binary();
        let uninstall_args = vec![&binary, "plugins", "uninstall", "example"];
        run(uninstall_args, None, None)?;
        Ok(())
    }

    // TODO: Test on Windows
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_cloud_plugin_install() -> Result<()> {
        // Create a temporary directory for plugin source and manifests
        let temp_dir = tempdir()?;
        let dir = temp_dir.path();
        let installed_plugins_dir = dir.join("tmp");

        // Ensure that spin installs the plugins into the temporary directory
        let mut env_map: HashMap<&str, &str> = HashMap::new();
        env_map.insert(
            "TEST_PLUGINS_DIRECTORY",
            installed_plugins_dir.to_str().unwrap(),
        );

        // `spin login --help` should cause the `cloud` plugin to be installed
        let spin_binary = spin_binary();
        let args = vec![&spin_binary, "login", "--help"];

        // Execute example plugin which writes "This is an example Spin plugin!" to a specified file
        let output = run(args, None, Some(env_map.clone()))?;

        // Ensure plugin is installed
        assert!(std::str::from_utf8(&output.stdout)?
            .trim()
            .contains("The `cloud` plugin is required. Installing now."));

        // Ensure login help info is displayed
        assert!(std::str::from_utf8(&output.stdout)?
            .trim()
            .contains("Log into Fermyon Cloud"));

        Ok(())
    }

    #[tokio::test]
    async fn test_build_command() -> Result<()> {
        do_test_build_command("tests/build/simple").await
    }

    #[tokio::test]
    async fn test_outbound_post() -> Result<()> {
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 0)).await?;

        let prefix = format!("http://{}", listener.local_addr()?);

        let body = Arc::new(Mutex::new(Vec::new()));

        let server = {
            let body = body.clone();
            async move {
                loop {
                    let (stream, _) = listener.accept().await?;
                    let body = body.clone();
                    task::spawn(async move {
                        if let Err(e) = http1::Builder::new()
                            .keep_alive(true)
                            .serve_connection(
                                TokioIo::new(stream),
                                service_fn(
                                    move |request: hyper::Request<hyper::body::Incoming>| {
                                        let body = body.clone();
                                        async move {
                                            if let &Method::POST = request.method() {
                                                let req_body = request.into_body();
                                                let bytes =
                                                    req_body.collect().await?.to_bytes().to_vec();
                                                *body.lock().unwrap() = bytes;
                                                Ok::<_, Error>(hyper::Response::new(body::empty()))
                                            } else {
                                                Ok(hyper::Response::builder()
                                                    .status(StatusCode::METHOD_NOT_ALLOWED)
                                                    .body(body::empty())?)
                                            }
                                        }
                                    },
                                ),
                            )
                            .await
                        {
                            log::warn!("{e:?}");
                        }
                    });

                    // Help rustc with type inference:
                    if false {
                        return Ok::<_, Error>(());
                    }
                }
            }
        }
        .then(|result| {
            if let Err(e) = result {
                log::warn!("{e:?}");
            }
            future::ready(())
        })
        .boxed();

        let (_tx, rx) = oneshot::channel::<()>();

        task::spawn(async move {
            drop(future::select(server, rx).await);
        });
        let controller = SpinTestController::with_manifest(
            "examples/http-rust-outbound-post/spin.toml",
            &[],
            &[],
        )
        .await?;

        let response = Client::new()
            .get(format!("http://{}/", controller.url))
            .header("url", format!("{prefix}/"))
            .send()
            .await?;
        assert_eq!(200, response.status());
        assert_eq!(b"Hello, world!", body.lock().unwrap().as_slice());

        Ok(())
    }

    #[tokio::test]
    async fn test_wasi_http_hash_all() -> Result<()> {
        let bodies = Arc::new(
            [
                ("/a", "’Twas brillig, and the slithy toves"),
                ("/b", "Did gyre and gimble in the wabe:"),
                ("/c", "All mimsy were the borogoves,"),
                ("/d", "And the mome raths outgrabe."),
            ]
            .into_iter()
            .collect::<HashMap<_, _>>(),
        );

        let listener = tokio::net::TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 0)).await?;

        let prefix = format!("http://{}", listener.local_addr()?);

        let server = {
            let bodies = bodies.clone();
            async move {
                loop {
                    let (stream, _) = listener.accept().await?;
                    let bodies = bodies.clone();
                    task::spawn(async move {
                        if let Err(e) = http1::Builder::new()
                            .keep_alive(true)
                            .serve_connection(
                                TokioIo::new(stream),
                                service_fn(move |request| {
                                    let bodies = bodies.clone();
                                    async move {
                                        if let (&Method::GET, Some(body)) =
                                            (request.method(), bodies.get(request.uri().path()))
                                        {
                                            Ok::<_, Error>(hyper::Response::new(body::full(
                                                Bytes::copy_from_slice(body.as_bytes()),
                                            )))
                                        } else {
                                            Ok(hyper::Response::builder()
                                                .status(StatusCode::METHOD_NOT_ALLOWED)
                                                .body(body::empty())?)
                                        }
                                    }
                                }),
                            )
                            .await
                        {
                            log::warn!("{e:?}");
                        }
                    });

                    // Help rustc with type inference:
                    if false {
                        return Ok::<_, Error>(());
                    }
                }
            }
        }
        .then(|result| {
            if let Err(e) = result {
                log::warn!("{e:?}");
            }
            future::ready(())
        })
        .boxed();

        let (_tx, rx) = oneshot::channel::<()>();

        task::spawn(async move {
            drop(future::select(server, rx).await);
        });

        let controller = SpinTestController::with_manifest(
            "examples/wasi-http-rust-streaming-outgoing-body/spin.toml",
            &[],
            &[],
        )
        .await?;

        let mut request = Client::new().get(format!("http://{}/hash-all", controller.url));
        for path in bodies.keys() {
            request = request.header("url", format!("{prefix}{path}"));
        }
        let response = request.send().await?;

        assert_eq!(200, response.status());
        let body = response.text().await?;
        for line in body.lines() {
            let (url, hash) = line
                .split_once(": ")
                .ok_or_else(|| anyhow!("expected string of form `<url>: <sha-256>`; got {line}"))?;

            let path = url
                .strip_prefix(&prefix)
                .ok_or_else(|| anyhow!("expected string with prefix {prefix}; got {url}"))?;

            let mut hasher = Sha256::new();
            hasher.update(
                bodies
                    .get(path)
                    .ok_or_else(|| anyhow!("unexpected path: {path}"))?,
            );
            assert_eq!(hash, hex::encode(hasher.finalize()));
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_wasi_http_echo() -> Result<()> {
        wasi_http_echo("echo", None).await
    }

    #[tokio::test]
    async fn test_wasi_http_double_echo() -> Result<()> {
        wasi_http_echo("double-echo", Some("echo")).await
    }

    async fn wasi_http_echo(uri: &str, url_header_uri: Option<&str>) -> Result<()> {
        let body = {
            // A sorta-random-ish megabyte
            let mut n = 0_u8;
            iter::repeat_with(move || {
                n = n.wrapping_add(251);
                n
            })
            .take(1024 * 1024)
            .collect::<Vec<_>>()
        };

        let controller = SpinTestController::with_manifest(
            "examples/wasi-http-rust-streaming-outgoing-body/spin.toml",
            &[],
            &[],
        )
        .await?;

        let mut request = Client::new()
            .post(format!("http://{}/{uri}", controller.url))
            .header("content-type", "application/octet-stream");

        if let Some(url_header_uri) = url_header_uri {
            request = request.header("url", format!("http://{}/{url_header_uri}", controller.url));
        }

        let response = request
            .body(reqwest::Body::wrap_stream(stream::iter(
                body.chunks(16 * 1024)
                    .map(|chunk| Ok::<_, Error>(Bytes::copy_from_slice(chunk)))
                    .collect::<Vec<_>>(),
            )))
            .send()
            .await?;

        assert_eq!(200, response.status());
        assert_eq!(
            response.headers()["content-type"],
            "application/octet-stream"
        );
        let received = response.bytes().await?;
        if body != received {
            panic!(
                "body content mismatch (expected length {}; actual length {})",
                body.len(),
                received.len()
            );
        }

        Ok(())
    }

    async fn test_wasi_http_rc(manifest_path: &str) -> Result<()> {
        let body = b"So rested he by the Tumtum tree";

        let listener = tokio::net::TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 0)).await?;

        let prefix = format!("http://{}", listener.local_addr()?);

        let server = {
            async move {
                loop {
                    let (stream, _) = listener.accept().await?;
                    task::spawn(async move {
                        if let Err(e) = http1::Builder::new()
                            .keep_alive(true)
                            .serve_connection(
                                TokioIo::new(stream),
                                service_fn(move |request| async move {
                                    if let &Method::GET = request.method() {
                                        Ok::<_, Error>(hyper::Response::new(body::full(
                                            Bytes::copy_from_slice(body),
                                        )))
                                    } else {
                                        Ok(hyper::Response::builder()
                                            .status(StatusCode::METHOD_NOT_ALLOWED)
                                            .body(body::empty())?)
                                    }
                                }),
                            )
                            .await
                        {
                            log::warn!("{e:?}");
                        }
                    });

                    // Help rustc with type inference:
                    if false {
                        return Ok::<_, Error>(());
                    }
                }
            }
        }
        .then(|result| {
            if let Err(e) = result {
                log::warn!("{e:?}");
            }
            future::ready(())
        })
        .boxed();

        let (_tx, rx) = oneshot::channel::<()>();

        task::spawn(async move {
            drop(future::select(server, rx).await);
        });

        let controller = SpinTestController::with_manifest(manifest_path, &[], &[]).await?;

        let response = Client::new()
            .get(format!("http://{}/", controller.url))
            .header("url", format!("{prefix}/"))
            .send()
            .await?;

        assert_eq!(200, response.status());
        assert_eq!(body as &[_], response.bytes().await?);

        Ok(())
    }

    #[tokio::test]
    async fn test_wasi_http_rc_11_10() -> Result<()> {
        test_wasi_http_rc("tests/http/wasi-http-rust-0.2.0-rc-2023-11-10/spin.toml").await
    }

    #[tokio::test]
    async fn test_wasi_http_rc_12_05() -> Result<()> {
        test_wasi_http_rc("tests/http/wasi-http-rust-0.2.0-rc-2023-12-05/spin.toml").await
    }

    /// Build an app whose component `workdir` is a subdirectory.
    #[tokio::test]
    #[cfg(not(tarpaulin))]
    async fn test_build_command_nested_workdir() -> Result<()> {
        do_test_build_command("tests/build/nested").await
    }

    /// Build an app whose component `workdir` is a sibling.
    #[tokio::test]
    #[cfg(not(tarpaulin))]
    async fn test_build_command_sibling_workdir() -> Result<()> {
        do_test_build_command("tests/build/sibling").await
    }
}
