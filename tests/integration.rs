#[cfg(test)]
mod integration_tests {
    use anyhow::{Context, Result};
    use hyper::{Body, Client, Response};
    use spin_loader::local::{
        config::{RawAppManifestAnyVersion, RawModuleSource},
        raw_manifest_from_file,
    };
    use std::{
        collections::HashMap,
        ffi::OsStr,
        fs,
        net::{Ipv4Addr, SocketAddrV4, TcpListener},
        path::Path,
        process::{self, Child, Command, Output},
        time::Duration,
    };
    use tempfile::tempdir;
    use tokio::{net::TcpStream, time::sleep};

    const RUST_HTTP_INTEGRATION_TEST: &str = "tests/http/simple-spin-rust";

    const DEFAULT_MANIFEST_LOCATION: &str = "spin.toml";

    const SPIN_BINARY: &str = "./target/debug/spin";

    // This module consist of all integration tests that require dependencies such as bindle-server, nomad, and Hippo.Web to be installed.
    #[cfg(feature = "e2e-tests")]
    mod e2e_tests {
        use super::*;
        use hyper::header::HeaderName;
        use std::path::PathBuf;
        use tempfile::TempDir;
        use which::which;

        const RUST_HTTP_INTEGRATION_TEST_REF: &str = "spin-hello-world/1.0.0";

        const RUST_HTTP_HEADERS_ENV_ROUTES_TEST: &str = "tests/http/headers-env-routes-test";
        const RUST_HTTP_HEADERS_ENV_ROUTES_TEST_REF: &str = "spin-headers-env-routes-test/1.0.0";

        const BINDLE_SERVER_BINARY: &str = "bindle-server";
        const NOMAD_BINARY: &str = "nomad";
        const HIPPO_BINARY: &str = "Hippo.Web";

        const BINDLE_SERVER_PATH_ENV: &str = "SPIN_TEST_BINDLE_SERVER_PATH";
        const BINDLE_SERVER_BASIC_AUTH_HTPASSWD_FILE: &str = "tests/http/htpasswd";
        const BINDLE_SERVER_BASIC_AUTH_USER: &str = "bindle-user";
        const BINDLE_SERVER_BASIC_AUTH_PASSWORD: &str = "topsecret";

        const HIPPO_BASIC_AUTH_USER: &str = "hippo-user";
        const HIPPO_BASIC_AUTH_PASSWORD: &str = "topsecret";

        // This assumes all tests have been previously compiled by the top-level build script.

        #[tokio::test]
        async fn test_dependencies() -> Result<()> {
            which(get_process(BINDLE_SERVER_BINARY))
                .with_context(|| format!("Can't find {}", get_process(BINDLE_SERVER_BINARY)))?;
            which(get_process(NOMAD_BINARY))
                .with_context(|| format!("Can't find {}", get_process(NOMAD_BINARY)))?;
            which(get_process(HIPPO_BINARY))
                .with_context(|| format!("Can't find {}", get_process(HIPPO_BINARY)))?;

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
                None,
            )?;

            // start Spin using the bindle reference of the application that was just pushed.
            let s = SpinTestController::with_bindle(RUST_HTTP_INTEGRATION_TEST_REF, &b.url, &[])
                .await?;

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
                None,
            )?;

            // start Spin using the bindle reference of the application that was just pushed.
            let s = SpinTestController::with_bindle(RUST_HTTP_INTEGRATION_TEST_REF, &b.url, &[])
                .await?;

            assert_status(&s, "/test/hello", 200).await?;
            assert_status(&s, "/test/hello/wildcards/should/be/handled", 200).await?;
            assert_status(&s, "/thisshouldfail", 404).await?;

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
                .join("simple_spin_rust.wasm");
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
                None,
            )?;

            let manifest_template =
                format!("{}/{}", RUST_HTTP_INTEGRATION_TEST, "spin-from-parcel.toml");
            let manifest = replace_text(&manifest_template, "AWAITING_PARCEL_SHA", &parcel_sha);

            let s = SpinTestController::with_manifest(
                &format!("{}", manifest.path.display()),
                &[],
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

        #[tokio::test]
        async fn test_spin_deploy() -> Result<()> {
            // start the Bindle registry.
            let config = BindleTestControllerConfig {
                basic_auth_enabled: false,
            };
            let _nomad = NomadTestController::new().await?;
            let bindle = BindleTestController::new(config).await?;
            let hippo = HippoTestController::new(&bindle.url).await?;

            // push the application to the registry using the Spin CLI.
            run(
                vec![
                    SPIN_BINARY,
                    "login",
                    "--bindle-server",
                    &bindle.url,
                    "--url",
                    &hippo.url,
                    "--username",
                    HIPPO_BASIC_AUTH_USER,
                    "--password",
                    HIPPO_BASIC_AUTH_PASSWORD,
                ],
                None,
                None,
            )?;
            run(
                vec![
                    SPIN_BINARY,
                    "deploy",
                    "--file",
                    &format!(
                        "{}/{}",
                        RUST_HTTP_HEADERS_ENV_ROUTES_TEST, DEFAULT_MANIFEST_LOCATION
                    ),
                ],
                None,
                None,
            )?;

            let apps_vm = hippo.client.list_apps().await?;
            assert_eq!(apps_vm.items.len(), 1, "hippo apps: {apps_vm:?}");

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
                                    format!(
                                        "executing {}: is binary on PATH?",
                                        bindle_server_binary
                                    )
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

        /// Controller for running Nomad.
        pub struct NomadTestController {
            pub url: String,
            nomad_handle: Child,
        }

        impl NomadTestController {
            pub async fn new() -> Result<NomadTestController> {
                let url = "127.0.0.1:4646".to_string();

                let mut nomad_handle = Command::new(get_process(NOMAD_BINARY))
                    .args(["agent", "-dev"])
                    .spawn()
                    .with_context(|| "executing nomad")?;

                wait_tcp(&url, &mut nomad_handle, NOMAD_BINARY).await?;

                Ok(Self { url, nomad_handle })
            }
        }

        impl Drop for NomadTestController {
            fn drop(&mut self) {
                let _ = self.nomad_handle.kill();
            }
        }

        /// Controller for running Hippo.
        pub struct HippoTestController {
            pub url: String,
            pub client: hippo::Client,
            hippo_handle: Child,
        }

        impl HippoTestController {
            pub async fn new(bindle_url: &str) -> Result<HippoTestController> {
                let url = format!("http://127.0.0.1:{}", get_random_port()?);

                let mut hippo_handle = Command::new(get_process(HIPPO_BINARY))
                    .env("ASPNETCORE_URLS", &url)
                    .env("Nomad__Driver", "raw_exec")
                    .env("Nomad__Datacenters__0", "dc1")
                    .env("Database__Driver", "inmemory")
                    .env("ConnectionStrings__Bindle", format!("Address={bindle_url}"))
                    .env("Jwt__Key", "ceci n'est pas une jeton")
                    .env("Jwt__Issuer", "localhost")
                    .env("Jwt__Audience", "localhost")
                    .spawn()
                    .with_context(|| "executing hippo")?;

                wait_hippo(&url, &mut hippo_handle, HIPPO_BINARY).await?;

                let client = hippo::Client::new(hippo::ConnectionInfo {
                    url: url.clone(),
                    danger_accept_invalid_certs: true,
                    api_key: None,
                });
                client
                    .register(
                        HIPPO_BASIC_AUTH_USER.into(),
                        HIPPO_BASIC_AUTH_PASSWORD.into(),
                    )
                    .await?;
                let token_info = client
                    .login(
                        HIPPO_BASIC_AUTH_USER.into(),
                        HIPPO_BASIC_AUTH_PASSWORD.into(),
                    )
                    .await?;
                let client = hippo::Client::new(hippo::ConnectionInfo {
                    url: url.clone(),
                    danger_accept_invalid_certs: true,
                    api_key: token_info.token,
                });

                Ok(Self {
                    url,
                    client,
                    hippo_handle,
                })
            }
        }

        impl Drop for HippoTestController {
            fn drop(&mut self) {
                let _ = self.hippo_handle.kill();
            }
        }

        async fn wait_hippo(url: &str, process: &mut Child, target: &str) -> Result<()> {
            println!("hippo url is {} and process is {:?}", url, process);
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

                if let Ok(rsp) = reqwest::get(format!("{url}/healthz")).await {
                    if rsp.status().is_success() {
                        break;
                    }
                }

                wait_count += 1;
                sleep(Duration::from_secs(1)).await;
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

    #[cfg(feature = "outbound-redis-tests")]
    mod outbound_redis_tests {
        use super::*;

        const RUST_OUTBOUND_REDIS_INTEGRATION_TEST: &str =
            "tests/outbound-redis/http-rust-outbound-redis";

        #[tokio::test]
        async fn test_outbound_redis_rust_local() -> Result<()> {
            let s = SpinTestController::with_manifest(
                &format!(
                    "{}/{}",
                    RUST_OUTBOUND_REDIS_INTEGRATION_TEST, DEFAULT_MANIFEST_LOCATION
                ),
                &[],
                &[],
                None,
            )
            .await?;

            assert_status(&s, "/test", 204).await?;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_simple_rust_local() -> Result<()> {
        let s = SpinTestController::with_manifest(
            &format!(
                "{}/{}",
                RUST_HTTP_INTEGRATION_TEST, DEFAULT_MANIFEST_LOCATION
            ),
            &[],
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

    #[cfg(feature = "outbound-pg-tests")]
    mod outbound_pg_tests {
        use super::*;

        const RUST_OUTBOUND_PG_INTEGRATION_TEST: &str = "tests/outbound-pg/http-rust-outbound-pg";

        #[tokio::test]
        async fn test_outbound_pg_rust_local() -> Result<()> {
            let s = SpinTestController::with_manifest(
                &format!(
                    "{}/{}",
                    RUST_OUTBOUND_PG_INTEGRATION_TEST, DEFAULT_MANIFEST_LOCATION
                ),
                &[],
                &[],
                None,
            )
            .await?;

            assert_status(&s, "/test_numeric_types", 200).await?;
            assert_status(&s, "/test_character_types", 200).await?;
            assert_status(&s, "/test_general_types", 200).await?;
            assert_status(&s, "/pg_backend_pid", 200).await?;

            Ok(())
        }
    }

    #[cfg(feature = "outbound-mysql-tests")]
    mod outbound_mysql_tests {
        use super::*;

        const RUST_OUTBOUND_MYSQL_INTEGRATION_TEST: &str =
            "tests/outbound-mysql/http-rust-outbound-mysql";

        #[tokio::test]
        async fn test_outbound_mysql_rust_local() -> Result<()> {
            let s = SpinTestController::with_manifest(
                &format!(
                    "{}/{}",
                    RUST_OUTBOUND_MYSQL_INTEGRATION_TEST, DEFAULT_MANIFEST_LOCATION
                ),
                &[],
                &[],
                None,
            )
            .await?;

            assert_status(&s, "/test_numeric_types", 200).await?;
            assert_status(&s, "/test_character_types", 200).await?;

            Ok(())
        }
    }

    #[cfg(feature = "config-provider-tests")]
    mod config_provider_tests {
        use super::*;

        const RUST_HTTP_VAULT_CONFIG_TEST: &str = "tests/http/vault-config-test";
        const VAULT_BINARY: &str = "vault";
        const VAULT_ROOT_TOKEN: &str = "root";

        #[tokio::test]
        async fn test_vault_config_provider() -> Result<()> {
            let vault = VaultTestController::new().await?;
            let http_client = reqwest::Client::new();
            let data = r#"
{
    "data": {
        "value": "test_password"
    }
}
"#;
            let body_map: HashMap<String, HashMap<String, String>> = serde_json::from_str(data)?;
            let status = http_client
                .post(format!("{}/v1/secret/data/password", &vault.url))
                .header("X-Vault-Token", VAULT_ROOT_TOKEN)
                .json(&body_map)
                .send()
                .await?
                .status();
            assert_eq!(status, 200);

            let s = SpinTestController::with_manifest(
                &format!(
                    "{}/{}",
                    RUST_HTTP_VAULT_CONFIG_TEST, DEFAULT_MANIFEST_LOCATION
                ),
                &[
                    "--runtime-config-file",
                    &format!("{}/{}", RUST_HTTP_VAULT_CONFIG_TEST, "runtime_config.toml"),
                ],
                &[],
                None,
            )
            .await?;

            assert_status(&s, "/", 200).await?;

            Ok(())
        }

        /// Controller for running Vault.
        pub struct VaultTestController {
            pub url: String,
            vault_handle: Child,
        }

        impl VaultTestController {
            pub async fn new() -> Result<VaultTestController> {
                let address = "127.0.0.1:8200";
                let url = format!("http://{}", address);

                let mut vault_handle = Command::new(get_process(VAULT_BINARY))
                    .args(["server", "-dev", "-dev-root-token-id", VAULT_ROOT_TOKEN])
                    .spawn()
                    .with_context(|| "executing vault")?;

                wait_vault(&url, &mut vault_handle, VAULT_BINARY).await?;

                Ok(Self { url, vault_handle })
            }
        }

        impl Drop for VaultTestController {
            fn drop(&mut self) {
                let _ = self.vault_handle.kill();
            }
        }

        async fn wait_vault(url: &str, process: &mut Child, target: &str) -> Result<()> {
            println!("vault url is {} and process is {:?}", url, process);
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

                let client = reqwest::Client::new();
                if let Ok(rsp) = client
                    .get(format!("{url}/v1/sys/health"))
                    .header("X-Vault-Token", VAULT_ROOT_TOKEN)
                    .send()
                    .await
                {
                    if rsp.status().is_success() {
                        break;
                    }
                }

                wait_count += 1;
                sleep(Duration::from_secs(1)).await;
            }

            Ok(())
        }
    }

    async fn assert_status(
        s: &SpinTestController,
        absolute_uri: &str,
        expected: u16,
    ) -> Result<()> {
        let res = req(s, absolute_uri).await?;
        let status = res.status();
        let body = hyper::body::to_bytes(res.into_body())
            .await
            .expect("read body");
        assert_eq!(status, expected, "{}", String::from_utf8_lossy(&body));

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
            spin_args: &[&str],
            spin_app_env: &[&str],
            bindle_url: Option<&str>,
        ) -> Result<SpinTestController> {
            // start Spin using the given application manifest and wait for the HTTP server to be available.
            let url = format!("127.0.0.1:{}", get_random_port()?);
            let mut args = vec!["up", "--file", manifest_path, "--listen", &url];
            args.extend(spin_args);
            if let Some(b) = bindle_url {
                args.push("--bindle-server");
                args.push(b);
            }
            for v in spin_app_env {
                args.push("--env");
                args.push(v);
            }

            let mut spin_handle = Command::new(get_process(SPIN_BINARY))
                .args(args)
                .env(
                    "RUST_LOG",
                    "spin=trace,spin_loader=trace,spin_core=trace,spin_http=trace",
                )
                .spawn()
                .with_context(|| "executing Spin")?;

            // ensure the server is accepting requests before continuing.
            wait_tcp(&url, &mut spin_handle, SPIN_BINARY).await?;

            Ok(SpinTestController { url, spin_handle })
        }

        // Unfortunately, this is a lot of duplicated code.
        #[cfg(feature = "e2e-tests")]
        pub async fn with_bindle(
            id: &str,
            bindle_url: &str,
            env: &[&str],
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

            let mut spin_handle = Command::new(get_process(SPIN_BINARY))
                .args(args)
                .env(
                    "RUST_LOG",
                    "spin=trace,spin_loader=trace,spin_core=trace,spin_http=trace",
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
        if let Some(envs) = envs {
            for (k, v) in envs {
                cmd.env(k, v);
            }
        }

        let output = cmd.output()?;
        let code = output.status.code().expect("should have status code");
        if code != 0 {
            println!("{:#?}", std::str::from_utf8(&output.stderr)?);
            println!("{:#?}", std::str::from_utf8(&output.stdout)?);
            panic!("command `{:?}` exited with code {}", cmd, code);
        }

        Ok(output)
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
        let manifest_file = dir.as_ref().join("spin.toml");
        let RawAppManifestAnyVersion::V1(manifest) = raw_manifest_from_file(&manifest_file).await?;

        let mut sources = vec![];
        for component_manifest in manifest.components.iter() {
            if let RawModuleSource::FileReference(file) = &component_manifest.source {
                sources.push(dir.as_ref().join(file));
            } else {
                panic!(
                    "{}.{}: source is not a file reference",
                    manifest.info.name, component_manifest.id
                )
            }
        }

        // Delete build output so that later it can be assumed: if the output
        // exists, it is because `spin build` succeeded.
        for source in sources.iter() {
            if source.exists() {
                std::fs::remove_file(source)?
            }
        }

        run(
            vec![
                SPIN_BINARY,
                "build",
                "--file",
                manifest_file.to_str().unwrap(),
            ],
            None,
            None,
        )?;

        let mut missing_sources_count = 0;
        for (i, source) in sources.iter().enumerate() {
            if source.exists() {
                std::fs::remove_file(source)?;
            } else {
                missing_sources_count += 1;
                println!(
                    "{}.{} source file was not generated by build",
                    manifest.info.name, manifest.components[i].id
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
trigger = { type = "http", base = "/" }
version = "0.1.0"
[[component]]
id = "unbuilt"
source = "DOES-NOT-EXIST.wasm"
[component.trigger]
route = "/..."
"#;

        std::fs::write(&manifest_file, toml_text)?;

        let up_help_args = vec![
            SPIN_BINARY,
            "up",
            "--file",
            manifest_file.to_str().unwrap(),
            "--help",
        ];

        let output = run(up_help_args, None, None)?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("--follow-all"));
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
        fs::write(
            &manifest_file_path,
            serde_json::to_string(&plugin_manifest_json).unwrap(),
        )?;

        // Install plugin
        let install_args = vec![
            SPIN_BINARY,
            "plugin",
            "install",
            "--file",
            manifest_file_path.to_str().unwrap(),
            "--yes",
        ];
        run(install_args, None, Some(env_map.clone()))?;

        // Execute example plugin which writes "This is an example Spin plugin!" to a specified file
        let execute_args = vec![SPIN_BINARY, "example"];
        let output = run(execute_args, None, Some(env_map.clone()))?;

        // Verify plugin successfully wrote to output file
        assert_eq!(
            std::str::from_utf8(&output.stdout)?.trim(),
            "This is an example Spin plugin!"
        );

        // Upgrade plugin to newer version
        *plugin_manifest_json.get_mut("version").unwrap() = serde_json::json!("0.2.1");
        fs::write(
            dir.join("example-plugin-manifest.json"),
            serde_json::to_string(&plugin_manifest_json).unwrap(),
        )?;
        let upgrade_args = vec![
            SPIN_BINARY,
            "plugin",
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
        let manifest = fs::read_to_string(installed_manifest)?;
        assert!(manifest.contains("0.2.1"));

        // Uninstall plugin
        let uninstall_args = vec![SPIN_BINARY, "plugin", "uninstall", "example"];
        run(uninstall_args, None, None)?;
        Ok(())
    }

    #[tokio::test]
    async fn test_build_command() -> Result<()> {
        do_test_build_command("tests/build/simple").await
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
