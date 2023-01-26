use anyhow::Result;

pub fn assert_status(url: &str, expected: u16) -> Result<()> {
    let resp = req(url)?;
    let status = resp.status();
    let body = resp.text()?;
    assert_eq!(status, expected, "{}", body);
    Ok(())
}

pub fn assert_http_request(
    url: &str,
    expected: u16,
    expected_headers: &[(&str, &str)],
    expected_body: Option<&str>,
) -> Result<()> {
    let res = req(url)?;

    let status = res.status();
    assert_eq!(expected, status.as_u16());

    let headers = res.headers();
    for (k, v) in expected_headers {
        assert_eq!(
            &headers
                .get(k.to_string())
                .unwrap_or_else(|| panic!("cannot find header {}", k))
                .to_str()?,
            v
        )
    }

    if let Some(expected_body_str) = expected_body {
        let body = &res.text()?;
        assert_eq!(expected_body_str, body);
    }

    Ok(())
}

fn req(url: &str) -> reqwest::Result<reqwest::blocking::Response> {
    println!("{}", url);
    reqwest::blocking::get(url)
}
