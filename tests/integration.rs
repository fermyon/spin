#[cfg(test)]
mod integration_tests {
    use anyhow::{anyhow, Context, Error, Result};
    use futures::{channel::oneshot, future, FutureExt};
    use http_body_util::BodyExt;
    use hyper::{body::Bytes, server::conn::http1, service::service_fn, Method, StatusCode};
    use hyper_util::rt::tokio::TokioIo;
    use reqwest::Client;
    use sha2::{Digest, Sha256};
    use spin_http::body;
    use std::{
        collections::HashMap,
        net::{Ipv4Addr, SocketAddrV4, TcpListener},
        path::Path,
        process::{Child, Command},
        sync::{Arc, Mutex},
        time::Duration,
    };
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

    fn get_process(binary: &str) -> String {
        if cfg!(target_os = "windows") {
            format!("{}.exe", binary)
        } else {
            binary.to_owned()
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
}
