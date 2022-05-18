#[cfg(test)]
mod integration_tests {
    use anyhow::{Context, Result};
    use hyper::{header::HeaderName, Body, Client, Response};
    use std::{
        ffi::OsStr,
        net::{Ipv4Addr, SocketAddrV4, TcpListener},
        path::{Path, PathBuf},
        process::{self, Child, Command},
        time::Duration,
    };
    use tempfile::TempDir;
    use tokio::{net::TcpStream, time::sleep};

    const RUST_HTTP_INTEGRATION_TEST: &str = "tests/http/simple-spin-rust";
    const RUST_HTTP_INTEGRATION_TEST_REF: &str = "spin-hello-world/1.0.0";

    const RUST_HTTP_STATIC_ASSETS_TEST: &str = "tests/http/assets-test";
    const RUST_HTTP_STATIC_ASSETS_REST_REF: &str = "spin-assets-test/1.0.0";

    const RUST_HTTP_HEADERS_ENV_ROUTES_TEST: &str = "tests/http/headers-env-routes-test";
    const RUST_HTTP_HEADERS_ENV_ROUTES_TEST_REF: &str = "spin-headers-env-routes-test/1.0.0";

    const DEFAULT_MANIFEST_LOCATION: &str = "spin.toml";

    const SPIN_BINARY: &str = "./target/debug/spin";
    const BINDLE_SERVER_BINARY: &str = "bindle-server";

    const BINDLE_SERVER_PATH_ENV: &str = "SPIN_TEST_BINDLE_SERVER_PATH";
    const BINDLE_SERVER_BASIC_AUTH_HTPASSWD_FILE: &str = "tests/http/htpasswd";
    const BINDLE_SERVER_BASIC_AUTH_USER: &str = "bindle-user";
    const BINDLE_SERVER_BASIC_AUTH_PASSWORD: &str = "topsecret";

    // This assumes all tests have been previously compiled by the top-level build script.

    #[tokio::test]
    async fn test_simple_rust_local() -> Result<()> {
        let s = SpinTestController::with_manifest(
            &format!(
                "{}/{}",
                RUST_HTTP_INTEGRATION_TEST, DEFAULT_MANIFEST_LOCATION
            ),
            &[],
            None,
        )
        .await?;

        assert_status(&s, "/test/hello", 200).await?;
        assert_status(&s, "/test/hello/wildcards/should/be/handled", 200).await?;
        assert_status(&s, "/thisshouldfail", 404).await?;
        assert_status(&s, "/test/hello/test-placement", 200).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_bindle_roundtrip_no_auth() -> Result<()> {
        // start the Bindle registry.
        let config = BindleTestControllerConfig {
            basic_auth_enabled: false,
        };
        let b = BindleTestController::new(config).await?;

        // push the application to the registry using the Spin CLI.
        run(
            vec![
                SPIN_BINARY,
                "bindle",
                "push",
                "--file",
                &format!(
                    "{}/{}",
                    RUST_HTTP_INTEGRATION_TEST, DEFAULT_MANIFEST_LOCATION
                ),
                "--bindle-server",
                &b.url,
            ],
            None,
        )?;

        // start Spin using the bindle reference of the application that was just pushed.
        let s =
            SpinTestController::with_bindle(RUST_HTTP_INTEGRATION_TEST_REF, &b.url, &[]).await?;

        assert_status(&s, "/test/hello", 200).await?;
        assert_status(&s, "/test/hello/wildcards/should/be/handled", 200).await?;
        assert_status(&s, "/thisshouldfail", 404).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_bindle_roundtrip_basic_auth() -> Result<()> {
        // start the Bindle registry.
        let config = BindleTestControllerConfig {
            basic_auth_enabled: true,
        };
        let b = BindleTestController::new(config).await?;

        // push the application to the registry using the Spin CLI.
        run(
            vec![
                SPIN_BINARY,
                "bindle",
                "push",
                "--file",
                &format!(
                    "{}/{}",
                    RUST_HTTP_INTEGRATION_TEST, DEFAULT_MANIFEST_LOCATION
                ),
                "--bindle-server",
                &b.url,
                "--bindle-username",
                BINDLE_SERVER_BASIC_AUTH_USER,
                "--bindle-password",
                BINDLE_SERVER_BASIC_AUTH_PASSWORD,
            ],
            None,
        )?;

        // start Spin using the bindle reference of the application that was just pushed.
        let s =
            SpinTestController::with_bindle(RUST_HTTP_INTEGRATION_TEST_REF, &b.url, &[]).await?;

        assert_status(&s, "/test/hello", 200).await?;
        assert_status(&s, "/test/hello/wildcards/should/be/handled", 200).await?;
        assert_status(&s, "/thisshouldfail", 404).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_bindle_static_assets() -> Result<()> {
        // start the Bindle registry.
        let config = BindleTestControllerConfig {
            basic_auth_enabled: false,
        };
        let b = BindleTestController::new(config).await?;

        // push the application to the registry using the Spin CLI.
        run(
            vec![
                SPIN_BINARY,
                "bindle",
                "push",
                "--file",
                &format!(
                    "{}/{}",
                    RUST_HTTP_STATIC_ASSETS_TEST, DEFAULT_MANIFEST_LOCATION
                ),
                "--bindle-server",
                &b.url,
            ],
            None,
        )?;

        // start Spin using the bindle reference of the application that was just pushed.
        let s =
            SpinTestController::with_bindle(RUST_HTTP_STATIC_ASSETS_REST_REF, &b.url, &[]).await?;

        assert_status(&s, "/static/thisshouldbemounted/1", 200).await?;
        assert_status(&s, "/static/thisshouldbemounted/2", 200).await?;
        assert_status(&s, "/static/thisshouldbemounted/3", 200).await?;

        assert_status(&s, "/static/donotmount/a", 404).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_can_cache_bindle_data() -> Result<()> {
        // start the Bindle registry.
        let config = BindleTestControllerConfig {
            basic_auth_enabled: false,
        };
        let b = BindleTestController::new(config).await?;
        let url = b.url.clone();

        // push the application to the registry using the Spin CLI.
        run(
            vec![
                SPIN_BINARY,
                "bindle",
                "push",
                "--file",
                &format!(
                    "{}/{}",
                    RUST_HTTP_STATIC_ASSETS_TEST, DEFAULT_MANIFEST_LOCATION
                ),
                "--bindle-server",
                &b.url,
            ],
            None,
        )?;

        let cache_dir = tempfile::tempdir()?;
        let cache_dir_text = format!("{}", cache_dir.path().display());

        // start Spin using the bindle reference of the application that was just pushed.
        let s = SpinTestController::with_bindle_opts(
            RUST_HTTP_STATIC_ASSETS_REST_REF,
            &url,
            &[],
            Some(&cache_dir_text),
        )
        .await?;

        assert_status(&s, "/static/thisshouldbemounted/1", 200).await?;
        assert_status(&s, "/static/thisshouldbemounted/2", 200).await?;
        assert_status(&s, "/static/thisshouldbemounted/3", 200).await?;

        assert_status(&s, "/static/donotmount/a", 404).await?;

        drop(s);
        drop(b);

        // The Bindle server is now unavailable but Spin should still be able to use the cache.
        let s = SpinTestController::with_bindle_opts(
            RUST_HTTP_STATIC_ASSETS_REST_REF,
            &url,
            &[],
            Some(&cache_dir_text),
        )
        .await?;

        assert_status(&s, "/static/thisshouldbemounted/1", 200).await?;
        assert_status(&s, "/static/thisshouldbemounted/2", 200).await?;
        assert_status(&s, "/static/thisshouldbemounted/3", 200).await?;

        assert_status(&s, "/static/donotmount/a", 404).await?;

        drop(cache_dir);

        Ok(())
    }

    #[tokio::test]
    async fn test_headers_env_routes() -> Result<()> {
        // start the Bindle registry.
        let config = BindleTestControllerConfig {
            basic_auth_enabled: false,
        };
        let b = BindleTestController::new(config).await?;

        // push the application to the registry using the Spin CLI.
        run(
            vec![
                SPIN_BINARY,
                "bindle",
                "push",
                "--file",
                &format!(
                    "{}/{}",
                    RUST_HTTP_HEADERS_ENV_ROUTES_TEST, DEFAULT_MANIFEST_LOCATION
                ),
                "--bindle-server",
                &b.url,
            ],
            None,
        )?;

        // start Spin using the bindle reference of the application that was just pushed.
        let s = SpinTestController::with_bindle(
            RUST_HTTP_HEADERS_ENV_ROUTES_TEST_REF,
            &b.url,
            &["foo=bar"],
        )
        .await?;

        assert_status(&s, "/env", 200).await?;

        verify_headers(
            &s,
            "/env/foo",
            200,
            &[("env_foo", "bar"), ("env_some_key", "some_value")],
            "/foo",
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_using_parcel_as_module_source() -> Result<()> {
        let wasm_path = PathBuf::from(RUST_HTTP_INTEGRATION_TEST)
            .join("target")
            .join("wasm32-wasi")
            .join("release")
            .join("spinhelloworld.wasm");
        let parcel_sha = file_digest_string(&wasm_path).expect("failed to get sha for parcel");

        // start the Bindle registry.
        let config = BindleTestControllerConfig {
            basic_auth_enabled: false,
        };
        let b = BindleTestController::new(config).await?;

        // push the application to the registry using the Spin CLI.
        run(
            vec![
                SPIN_BINARY,
                "bindle",
                "push",
                "--file",
                &format!(
                    "{}/{}",
                    RUST_HTTP_INTEGRATION_TEST, DEFAULT_MANIFEST_LOCATION
                ),
                "--bindle-server",
                &b.url,
            ],
            None,
        )?;

        let manifest_template =
            format!("{}/{}", RUST_HTTP_INTEGRATION_TEST, "spin-from-parcel.toml");
        let manifest = replace_text(&manifest_template, "AWAITING_PARCEL_SHA", &parcel_sha);

        let s = SpinTestController::with_manifest(
            &format!("{}", manifest.path.display()),
            &[],
            Some(&b.url),
        )
        .await?;

        assert_status(&s, "/test/hello", 200).await?;
        assert_status(&s, "/test/hello/wildcards/should/be/handled", 200).await?;
        assert_status(&s, "/thisshouldfail", 404).await?;
        assert_status(&s, "/test/hello/test-placement", 200).await?;

        Ok(())
    }

    async fn verify_headers(
        s: &SpinTestController,
        absolute_uri: &str,
        expected: u16,
        expected_env_as_headers: &[(&str, &str)],
        expected_path_info: &str,
    ) -> Result<()> {
        let res = req(s, absolute_uri).await?;
        assert_eq!(res.status(), expected);

        // check the environment variables sent back as headers:
        for (k, v) in expected_env_as_headers {
            assert_eq!(
                &res.headers()
                    .get(HeaderName::from_bytes(k.as_bytes())?)
                    .unwrap_or_else(|| panic!("cannot find header {}", k))
                    .to_str()?,
                v
            );
        }

        assert_eq!(
            res.headers()
                .get(HeaderName::from_bytes("spin-path-info".as_bytes())?)
                .unwrap_or_else(|| panic!("cannot find spin-path-info header"))
                .to_str()?,
            expected_path_info
        );

        Ok(())
    }

    async fn assert_status(
        s: &SpinTestController,
        absolute_uri: &str,
        expected: u16,
    ) -> Result<()> {
        let res = req(s, absolute_uri).await?;
        assert_eq!(res.status(), expected);

        Ok(())
    }

    async fn req(s: &SpinTestController, absolute_uri: &str) -> Result<Response<Body>> {
        let c = Client::new();
        let url = format!("http://{}{}", s.url, absolute_uri)
            .parse()
            .with_context(|| "cannot parse URL")?;
        Ok(c.get(url).await?)
    }

    /// Controller for running Spin.
    pub struct SpinTestController {
        pub url: String,
        spin_handle: Child,
    }

    impl SpinTestController {
        pub async fn with_manifest(
            manifest_path: &str,
            env: &[&str],
            bindle_url: Option<&str>,
        ) -> Result<SpinTestController> {
            // start Spin using the given application manifest and wait for the HTTP server to be available.
            let url = format!("127.0.0.1:{}", get_random_port()?);
            let mut args = vec!["up", "--file", manifest_path, "--listen", &url];
            if let Some(b) = bindle_url {
                args.push("--bindle-server");
                args.push(b);
            }
            for v in env {
                args.push("--env");
                args.push(v);
            }

            let mut spin_handle = Command::new(get_process(SPIN_BINARY))
                .args(args)
                .env(
                    "RUST_LOG",
                    "spin=trace,spin_loader=trace,spin_engine=trace,spin_http=trace",
                )
                .spawn()
                .with_context(|| "executing Spin")?;

            // ensure the server is accepting requests before continuing.
            wait_tcp(&url, &mut spin_handle, SPIN_BINARY).await?;

            Ok(SpinTestController { url, spin_handle })
        }

        pub async fn with_bindle(
            id: &str,
            bindle_url: &str,
            env: &[&str],
        ) -> Result<SpinTestController> {
            Self::with_bindle_opts(id, bindle_url, env, None).await
        }

        // Unfortunately, this is a lot of duplicated code.
        pub async fn with_bindle_opts(
            id: &str,
            bindle_url: &str,
            env: &[&str],
            cache_dir: Option<&str>,
        ) -> Result<SpinTestController> {
            let url = format!("127.0.0.1:{}", get_random_port()?);
            let mut args = vec![
                "up",
                "--bindle",
                id,
                "--bindle-server",
                bindle_url,
                "--listen",
                &url,
            ];
            for v in env {
                args.push("--env");
                args.push(v);
            }
            if let Some(d) = cache_dir {
                args.push("--bindle-cache-dir");
                args.push(d);
            }

            let mut spin_handle = Command::new(get_process(SPIN_BINARY))
                .args(args)
                .env(
                    "RUST_LOG",
                    "spin=trace,spin_loader=trace,spin_engine=trace,spin_http=trace",
                )
                .spawn()
                .with_context(|| "executing Spin")?;

            // ensure the server is accepting requests before continuing.
            wait_tcp(&url, &mut spin_handle, SPIN_BINARY).await?;

            Ok(SpinTestController { url, spin_handle })
        }
    }

    impl Drop for SpinTestController {
        fn drop(&mut self) {
            let _ = self.spin_handle.kill();
        }
    }

    /// Controller for running a Bindle server.
    /// This assumes `bindle-server` is present in the path.
    pub struct BindleTestController {
        pub url: String,
        pub server_cache: TempDir,
        server_handle: Child,
    }

    /// Config for the BindleTestController
    pub struct BindleTestControllerConfig {
        pub basic_auth_enabled: bool,
    }

    impl BindleTestController {
        pub async fn new(config: BindleTestControllerConfig) -> Result<BindleTestController> {
            let server_cache = tempfile::tempdir()?;

            let address = format!("127.0.0.1:{}", get_random_port()?);
            let url = format!("http://{}/v1/", address);

            let bindle_server_binary = std::env::var(BINDLE_SERVER_PATH_ENV)
                .unwrap_or_else(|_| BINDLE_SERVER_BINARY.to_owned());

            let auth_args = match config.basic_auth_enabled {
                true => vec!["--htpasswd-file", BINDLE_SERVER_BASIC_AUTH_HTPASSWD_FILE],
                false => vec!["--unauthenticated"],
            };

            let server_handle_result = Command::new(&bindle_server_binary)
                .args(
                    [
                        &[
                            "-d",
                            server_cache.path().to_string_lossy().to_string().as_str(),
                            "-i",
                            address.as_str(),
                        ],
                        auth_args.as_slice(),
                    ]
                    .concat(),
                )
                .spawn();

            let mut server_handle = match server_handle_result {
                Ok(h) => Ok(h),
                Err(e) => {
                    let is_path_explicit = std::env::var(BINDLE_SERVER_PATH_ENV).is_ok();
                    let context = match e.kind() {
                        std::io::ErrorKind::NotFound => {
                            if is_path_explicit {
                                format!(
                                    "executing {}: is the path/filename correct?",
                                    bindle_server_binary
                                )
                            } else {
                                format!("executing {}: is binary on PATH?", bindle_server_binary)
                            }
                        }
                        _ => format!("executing {}", bindle_server_binary),
                    };
                    Err(e).context(context)
                }
            }?;

            wait_tcp(&address, &mut server_handle, BINDLE_SERVER_BINARY).await?;

            Ok(Self {
                url,
                server_handle,
                server_cache,
            })
        }
    }

    impl Drop for BindleTestController {
        fn drop(&mut self) {
            let _ = self.server_handle.kill();
        }
    }

    fn run<S: Into<String> + AsRef<OsStr>>(args: Vec<S>, dir: Option<S>) -> Result<()> {
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

        let output = cmd.output()?;
        let code = output.status.code().expect("should have status code");
        if code != 0 {
            println!("{:#?}", std::str::from_utf8(&output.stderr)?);
            println!("{:#?}", std::str::from_utf8(&output.stdout)?);
            panic!("command `{:?}` exited with code {}", cmd, code);
        }

        Ok(())
    }

    fn get_process(binary: &str) -> String {
        if cfg!(target_os = "windows") {
            format!("{}.exe", binary)
        } else {
            binary.to_string()
        }
    }

    fn get_os_process() -> String {
        if cfg!(target_os = "windows") {
            String::from("powershell.exe")
        } else {
            String::from("/bin/bash")
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
            if wait_count >= 120 {
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
                Err(_) => {
                    wait_count += 1;
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }

        Ok(())
    }

    fn file_digest_string(path: impl AsRef<Path>) -> Result<String> {
        use sha2::{Digest, Sha256};
        let mut file = std::fs::File::open(&path)?;
        let mut sha = Sha256::new();
        std::io::copy(&mut file, &mut sha)?;
        let digest_value = sha.finalize();
        let digest_string = format!("{:x}", digest_value);
        Ok(digest_string)
    }

    struct AutoDeleteFile {
        pub path: PathBuf,
    }

    fn replace_text(template_path: impl AsRef<Path>, from: &str, to: &str) -> AutoDeleteFile {
        let dest = template_path.as_ref().with_extension("temp");
        let source_text =
            std::fs::read_to_string(template_path).expect("failed to read manifest template");
        let result_text = source_text.replace(from, to);
        std::fs::write(&dest, result_text).expect("failed to write temp manifest");
        AutoDeleteFile { path: dest }
    }

    impl Drop for AutoDeleteFile {
        fn drop(&mut self) {
            std::fs::remove_file(&self.path).unwrap();
        }
    }
}
