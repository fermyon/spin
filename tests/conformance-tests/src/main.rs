use anyhow::Context as _;

fn main() {
    let tests_dir = download_tests();

    for entry in std::fs::read_dir(tests_dir).unwrap() {
        let entry = entry.unwrap();
        if !entry.path().is_dir() {
            continue;
        }
        let spin_binary = "/Users/rylev/.local/bin/spin".into();
        let test_config = std::fs::read_to_string(entry.path().join("test.json5")).unwrap();
        let test_config: TestConfig = json5::from_str(&test_config).unwrap();
        let env_config = testing_framework::TestEnvironmentConfig::spin(
            spin_binary,
            [],
            move |e| {
                e.copy_into(entry.path().join("target"), "target")
                    .context("failed to copy target directory")?;
                e.copy_into(entry.path().join("spin.toml"), "spin.toml")
                    .context("failed to copy spin.toml")?;
                Ok(())
            },
            testing_framework::ServicesConfig::none(),
            testing_framework::runtimes::SpinAppType::Http,
        );
        let mut env = testing_framework::TestEnvironment::up(env_config, |_| Ok(())).unwrap();
        let spin = env.runtime_mut();
        for invocation in test_config.invocations {
            let Invocation::Http(invocation) = invocation;
            let headers = invocation
                .request
                .headers
                .iter()
                .map(|h| (h.name.as_str(), h.value.as_str()))
                .collect::<Vec<_>>();
            let request = testing_framework::http::Request::full(
                invocation.request.method.into(),
                &invocation.request.path,
                &headers,
                invocation.request.body,
            );
            let response = spin.make_http_request(request).unwrap();
            let stderr = spin.stderr();
            let body = String::from_utf8(response.body())
                .unwrap_or_else(|_| String::from("invalid utf-8"));
            assert_eq!(
                response.status(),
                invocation.response.status,
                "request to Spin failed\nstderr:\n{stderr}\nbody:\n{body}",
            );

            let mut actual_headers = response
                .headers()
                .iter()
                .map(|(k, v)| (k.to_lowercase(), v.to_lowercase()))
                .collect::<std::collections::HashMap<_, _>>();
            for expected_header in invocation.response.headers {
                let expected_name = expected_header.name.to_lowercase();
                let expected_value = expected_header.value.map(|v| v.to_lowercase());
                let actual_value = actual_headers.remove(&expected_name);
                let Some(actual_value) = actual_value.as_deref() else {
                    if expected_header.optional {
                        continue;
                    } else {
                        panic!(
                            "expected header {name} not found in response",
                            name = expected_header.name
                        )
                    }
                };
                if let Some(expected_value) = expected_value {
                    assert_eq!(actual_value, expected_value);
                }
            }
            if !actual_headers.is_empty() {
                panic!("unexpected headers: {actual_headers:?}");
            }

            if let Some(expected_body) = invocation.response.body {
                assert_eq!(body, expected_body);
            }
        }
    }
}

/// Download the conformance tests and return the path to the directory where they are written to
fn download_tests() -> std::path::PathBuf {
    let response = reqwest::blocking::get(
        "https://github.com/fermyon/conformance-tests/releases/download/canary/tests.tar.gz",
    )
    .unwrap()
    .error_for_status()
    .unwrap();
    let response = flate2::read::GzDecoder::new(response);
    let dir = std::env::temp_dir().join("conformance-tests");
    for entry in tar::Archive::new(response).entries().unwrap() {
        let mut entry = entry.unwrap();
        if entry.header().entry_type() != tar::EntryType::Regular {
            continue;
        }
        let path = dir.join(entry.path().unwrap());
        let parent_dir = path.parent().unwrap();
        std::fs::create_dir_all(&parent_dir).unwrap();
        let mut file = std::fs::File::create(&path).unwrap();
        std::io::copy(&mut entry, &mut file).unwrap();
    }
    dir
}

/// The configuration of a conformance test
#[derive(Debug, serde::Deserialize)]
struct TestConfig {
    invocations: Vec<Invocation>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum Invocation {
    Http(HttpInvocation),
}

/// An invocation of the runtime
#[derive(Debug, serde::Deserialize)]
struct HttpInvocation {
    request: Request,
    response: Response,
}

#[derive(Debug, serde::Deserialize)]
struct Request {
    #[serde(default)]
    method: Method,
    path: String,
    #[serde(default)]
    headers: Vec<RequestHeader>,
    #[serde(default)]
    body: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct Response {
    #[serde(default = "default_status")]
    status: u16,
    headers: Vec<ResponseHeader>,
    body: Option<String>,
}
#[derive(Debug, serde::Deserialize)]
struct RequestHeader {
    name: String,
    value: String,
}

#[derive(Debug, serde::Deserialize)]
struct ResponseHeader {
    name: String,
    value: Option<String>,
    #[serde(default)]
    optional: bool,
}

#[derive(Debug, serde::Deserialize, Default)]
enum Method {
    #[default]
    GET,
    POST,
}

impl From<Method> for testing_framework::http::Method {
    fn from(method: Method) -> Self {
        match method {
            Method::GET => testing_framework::http::Method::GET,
            Method::POST => testing_framework::http::Method::POST,
        }
    }
}

fn default_status() -> u16 {
    200
}
