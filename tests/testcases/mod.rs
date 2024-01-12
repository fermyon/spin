use anyhow::Context;
use redis::Commands;
use reqwest::header::HeaderValue;
use std::path::PathBuf;

/// Run an e2e test
pub fn run_test(
    test_name: impl Into<String>,
    mode: testing_framework::SpinMode,
    spin_up_args: impl IntoIterator<Item = String>,
    services_config: testing_framework::ServicesConfig,
    test: impl FnOnce(
            &mut testing_framework::TestEnvironment<testing_framework::Spin>,
        ) -> testing_framework::TestResult<anyhow::Error>
        + 'static,
) -> testing_framework::TestResult<anyhow::Error> {
    let mut env = bootstap_env(test_name, spin_up_args, services_config, mode)?;
    test(&mut env)?;
    Ok(())
}

/// Bootstrap a test environment
pub fn bootstap_env(
    test_name: impl Into<String>,
    spin_up_args: impl IntoIterator<Item = String>,
    services_config: testing_framework::ServicesConfig,
    mode: testing_framework::SpinMode,
) -> anyhow::Result<testing_framework::TestEnvironment<testing_framework::Spin>> {
    let test_name = test_name.into();
    let config = testing_framework::TestEnvironmentConfig::spin(
        spin_binary(),
        spin_up_args,
        move |env| preboot(&test_name, env),
        services_config,
        mode,
    );
    testing_framework::TestEnvironment::up(config)
}

/// Assert that a request to the spin server returns the expected status and body
pub fn assert_spin_request(
    spin: &mut testing_framework::Spin,
    request: testing_framework::Request<'_>,
    expected_status: u16,
    expected_headers: &[(&str, &str)],
    expected_body: &str,
) -> testing_framework::TestResult<anyhow::Error> {
    let uri = request.uri;
    let mut r = spin.make_http_request(request)?;
    let status = r.status();
    let headers = std::mem::take(r.headers_mut());
    let body = r.text().unwrap_or_else(|_| String::from("<non-utf8>"));
    if status != expected_status {
        return Err(testing_framework::TestError::Failure(anyhow::anyhow!(
            "Expected status {expected_status} for {uri} but got {status}\nBody:\n{body}",
        )));
    }
    let wrong_headers: std::collections::HashMap<_, _> = expected_headers
        .iter()
        .copied()
        .filter(|(ek, ev)| headers.get(*ek) != Some(&HeaderValue::from_str(ev).unwrap()))
        .collect();
    if !wrong_headers.is_empty() {
        return Err(testing_framework::TestError::Failure(anyhow::anyhow!(
            "Expected headers {headers:?}  to contain {wrong_headers:?}\nBody:\n{body}"
        )));
    }
    if body != expected_body {
        return Err(testing_framework::TestError::Failure(anyhow::anyhow!(
            "expected body '{expected_body}', got '{body}'"
        )));
    }
    Ok(())
}

/// Get the test environment ready to run a test
fn preboot(
    test: &str,
    env: &mut testing_framework::TestEnvironment<testing_framework::Spin>,
) -> anyhow::Result<()> {
    // Copy everything into the test environment
    env.copy_into(format!("tests/testcases/{test}"), "")?;

    // Copy the manifest with all templates substituted
    let manifest_path = PathBuf::from(format!("tests/testcases/{test}/spin.toml"));
    let mut template = testing_framework::ManifestTemplate::from_file(manifest_path)?;
    template.substitute(env)?;
    env.write_file("spin.toml", template.contents())?;
    Ok(())
}

/// Run a smoke test against a `spin new` http template
pub fn http_smoke_test_template(
    template_name: &str,
    template_url: Option<&str>,
    plugins: &[&str],
    prebuild_hook: impl FnOnce(&mut testing_framework::TestEnvironment<()>) -> anyhow::Result<()>,
    expected_body: &str,
) -> anyhow::Result<()> {
    http_smoke_test_template_with_route(
        template_name,
        template_url,
        plugins,
        prebuild_hook,
        "/",
        expected_body,
    )
}

/// Run a smoke test against a given http route for a `spin new` http template
pub fn http_smoke_test_template_with_route(
    template_name: &str,
    template_url: Option<&str>,
    plugins: &[&str],
    prebuild_hook: impl FnOnce(&mut testing_framework::TestEnvironment<()>) -> anyhow::Result<()>,
    route: &str,
    expected_body: &str,
) -> anyhow::Result<()> {
    let mut env = bootstrap_smoke_test(
        &testing_framework::ServicesConfig::none(),
        template_url,
        plugins,
        template_name,
        |_| Ok(Vec::new()),
        prebuild_hook,
        |_| Ok(Vec::new()),
        testing_framework::SpinMode::Http,
    )?;

    assert_spin_request(
        env.runtime_mut(),
        testing_framework::Request::new(reqwest::Method::GET, route),
        200,
        &[],
        expected_body,
    )?;

    Ok(())
}

/// Run a smoke test for a `spin new` redis template
pub fn redis_smoke_test_template(
    template_name: &str,
    template_url: Option<&str>,
    plugins: &[&str],
    new_app_args: impl FnOnce(u16) -> Vec<String>,
    prebuild_hook: impl FnOnce(&mut testing_framework::TestEnvironment<()>) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut env = bootstrap_smoke_test(
        &testing_framework::ServicesConfig::new(vec!["redis".into()])?,
        template_url,
        plugins,
        template_name,
        |env| {
            let redis_port = env
                .services_mut()
                .get_port(6379)?
                .context("no redis port was exposed by test services")?;
            Ok(new_app_args(redis_port))
        },
        prebuild_hook,
        |_| Ok(Vec::new()),
        testing_framework::SpinMode::Redis,
    )?;
    let redis_port = env
        .get_port(6379)?
        .context("no redis port was exposed by test services")?;
    let mut client = redis::Client::open(format!("redis://localhost:{redis_port}"))?;
    let mut conn = client.get_connection()?;
    let mut pubsub = conn.as_pubsub();
    pubsub.subscribe("redis-channel")?;
    client.publish("redis-channel", "hello from redis")?;

    // Wait for the message to be received (as an approximation for when Spin receives the message)
    let _ = pubsub.get_message()?;
    // Leave some time for the message to be processed
    std::thread::sleep(std::time::Duration::from_millis(100));

    let stderr = env.runtime_mut().stderr();
    assert!(stderr.contains("hello from redis"));

    Ok(())
}

static TEMPLATE_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Bootstrap a test environment for a smoke test
// TODO: refactor this function to not take so many arguments
#[allow(clippy::too_many_arguments)]
pub fn bootstrap_smoke_test(
    services: &testing_framework::ServicesConfig,
    template_url: Option<&str>,
    plugins: &[&str],
    template_name: &str,
    new_app_args: impl FnOnce(
        &mut testing_framework::TestEnvironment<()>,
    ) -> anyhow::Result<Vec<String>>,
    prebuild_hook: impl FnOnce(&mut testing_framework::TestEnvironment<()>) -> anyhow::Result<()>,
    spin_up_args: impl FnOnce(
        &mut testing_framework::TestEnvironment<()>,
    ) -> anyhow::Result<Vec<String>>,
    spin_mode: testing_framework::SpinMode,
) -> anyhow::Result<testing_framework::TestEnvironment<testing_framework::Spin>> {
    let mut env: testing_framework::TestEnvironment<()> =
        testing_framework::TestEnvironment::boot(services)?;

    let template_url = template_url.unwrap_or("https://github.com/fermyon/spin");
    let mut template_install = std::process::Command::new(spin_binary());
    template_install.args(["templates", "install", "--git", template_url, "--update"]);
    // We need to serialize template installs since they can't be run in parallel
    {
        let _guard = TEMPLATE_MUTEX.lock().unwrap();
        env.run_in(&mut template_install)?;
    }

    if !plugins.is_empty() {
        let mut plugin_update = std::process::Command::new(spin_binary());
        plugin_update.args(["plugin", "update"]);
        if let Err(e) = env.run_in(&mut plugin_update) {
            // We treat plugin updates as best efforts since it only needs to be run once
            if !e
                .to_string()
                .contains("update operation is already in progress")
            {
                return Err(e);
            }
        }
    }
    for plugin in plugins {
        let mut plugin_install = std::process::Command::new(spin_binary());
        plugin_install.args(["plugin", "install", plugin, "--yes"]);
        env.run_in(&mut plugin_install)?;
    }
    let mut new_app = std::process::Command::new(spin_binary());
    new_app
        .args([
            "new",
            "test",
            "-t",
            template_name,
            "-o",
            ".",
            "--accept-defaults",
        ])
        .args(new_app_args(&mut env)?);
    env.run_in(&mut new_app)?;
    prebuild_hook(&mut env)?;
    let mut build = std::process::Command::new(spin_binary());
    build.args(["build"]);
    env.run_in(&mut build)?;
    let spin_up_args = spin_up_args(&mut env)?;
    let spin = testing_framework::Spin::start(&spin_binary(), &env, spin_up_args, spin_mode)?;
    let env = env.start_runtime(spin)?;
    Ok(env)
}

/// Get the spin binary path
pub fn spin_binary() -> PathBuf {
    env!("CARGO_BIN_EXE_spin").into()
}
