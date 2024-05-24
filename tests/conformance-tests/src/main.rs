fn main() {
    let tests_dir = conformance_tests::download_tests().unwrap();

    for test in conformance_tests::tests(&tests_dir).unwrap() {
        let spin_binary = "/Users/rylev/.local/bin/spin".into();
        let env_config = testing_framework::TestEnvironmentConfig::spin(
            spin_binary,
            [],
            move |e| {
                e.copy_into(&test.manifest, "spin.toml")?;
                e.copy_into(&test.component, test.component.file_name().unwrap())?;
                Ok(())
            },
            testing_framework::ServicesConfig::none(),
            testing_framework::runtimes::SpinAppType::Http,
        );
        let mut env = testing_framework::TestEnvironment::up(env_config, |_| Ok(())).unwrap();
        let spin = env.runtime_mut();
        for invocation in test.config.invocations {
            let conformance_tests::config::Invocation::Http(invocation) = invocation;
            let headers = invocation
                .request
                .headers
                .iter()
                .map(|h| (h.name.as_str(), h.value.as_str()))
                .collect::<Vec<_>>();
            let request = testing_framework::http::Request::full(
                match invocation.request.method {
                    conformance_tests::config::Method::GET => testing_framework::http::Method::GET,
                    conformance_tests::config::Method::POST => {
                        testing_framework::http::Method::POST
                    }
                },
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
