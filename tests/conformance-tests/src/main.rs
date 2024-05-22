use std::collections::HashMap;

use anyhow::Context as _;

fn main() {
    let dir = std::fs::read_dir("/Users/rylev/Code/fermyon/conformance-test/tests").unwrap();
    for entry in dir {
        let entry = entry.unwrap();
        let spin_binary = "/Users/rylev/.local/bin/spin".into();
        let test_config = std::fs::read_to_string(entry.path().join("test.json5")).unwrap();
        let test_config: TestConfig = json5::from_str(&test_config).unwrap();
        let config = testing_framework::TestEnvironmentConfig::spin(
            spin_binary,
            ["-f".into(), entry.path().to_str().unwrap().into()],
            move |e| {
                e.copy_into(entry.path().join("spin.toml"), "spin.toml")
                    .context("failed to copy spin.toml")?;
                let mut cmd = std::process::Command::new("spin");
                cmd.env("CARGO_TARGET_DIR", entry.path().join("target"));
                cmd.args(["build", "-f", entry.path().to_str().unwrap()]);
                e.run_in(&mut cmd)?;
                Ok(())
            },
            testing_framework::ServicesConfig::none(),
            testing_framework::runtimes::SpinAppType::Http,
        );
        let mut env = testing_framework::TestEnvironment::up(config, |_| Ok(())).unwrap();
        let spin = env.runtime_mut();
        for invocation in test_config.invocations {
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
                .collect::<HashMap<_, _>>();
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

#[derive(Debug, serde::Deserialize)]
struct TestConfig {
    invocations: Vec<Invocation>,
}

#[derive(Debug, serde::Deserialize)]
struct Invocation {
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
