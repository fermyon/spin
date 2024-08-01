use std::{net::SocketAddr, str, str::FromStr};

use anyhow::Result;
use http::Uri;
use hyper::Request;
use spin_http::routes::RouteMatch;
use spin_outbound_networking::is_service_chaining_host;

use crate::Body;

// We need to make the following pieces of information available to both executors.
// While the values we set are identical, the way they are passed to the
// modules is going to be different, so each executor must must use the info
// in its standardized way (environment variables for the Wagi executor, and custom headers
// for the Spin HTTP executor).
pub const FULL_URL: [&str; 2] = ["SPIN_FULL_URL", "X_FULL_URL"];
pub const PATH_INFO: [&str; 2] = ["SPIN_PATH_INFO", "PATH_INFO"];
pub const MATCHED_ROUTE: [&str; 2] = ["SPIN_MATCHED_ROUTE", "X_MATCHED_ROUTE"];
pub const COMPONENT_ROUTE: [&str; 2] = ["SPIN_COMPONENT_ROUTE", "X_COMPONENT_ROUTE"];
pub const RAW_COMPONENT_ROUTE: [&str; 2] = ["SPIN_RAW_COMPONENT_ROUTE", "X_RAW_COMPONENT_ROUTE"];
pub const BASE_PATH: [&str; 2] = ["SPIN_BASE_PATH", "X_BASE_PATH"];
pub const CLIENT_ADDR: [&str; 2] = ["SPIN_CLIENT_ADDR", "X_CLIENT_ADDR"];

pub fn compute_default_headers(
    uri: &Uri,
    host: &str,
    route_match: &RouteMatch,
    client_addr: SocketAddr,
) -> anyhow::Result<Vec<([String; 2], String)>> {
    fn owned(strs: &[&'static str; 2]) -> [String; 2] {
        [strs[0].to_owned(), strs[1].to_owned()]
    }

    let owned_full_url: [String; 2] = owned(&FULL_URL);
    let owned_path_info: [String; 2] = owned(&PATH_INFO);
    let owned_matched_route: [String; 2] = owned(&MATCHED_ROUTE);
    let owned_component_route: [String; 2] = owned(&COMPONENT_ROUTE);
    let owned_raw_component_route: [String; 2] = owned(&RAW_COMPONENT_ROUTE);
    let owned_base_path: [String; 2] = owned(&BASE_PATH);
    let owned_client_addr: [String; 2] = owned(&CLIENT_ADDR);

    let mut res = vec![];
    let abs_path = uri
        .path_and_query()
        .expect("cannot get path and query")
        .as_str();

    let path_info = route_match.trailing_wildcard();

    let scheme = uri.scheme_str().unwrap_or("http");

    let full_url = format!("{}://{}{}", scheme, host, abs_path);

    res.push((owned_path_info, path_info));
    res.push((owned_full_url, full_url));
    res.push((owned_matched_route, route_match.based_route().to_string()));

    res.push((owned_base_path, "/".to_string()));
    res.push((
        owned_raw_component_route,
        route_match.raw_route().to_string(),
    ));
    res.push((owned_component_route, route_match.raw_route_or_prefix()));
    res.push((owned_client_addr, client_addr.to_string()));

    for (wild_name, wild_value) in route_match.named_wildcards() {
        let wild_header = format!("SPIN_PATH_MATCH_{}", wild_name.to_ascii_uppercase()); // TODO: safer
        let wild_wagi_header = format!("X_PATH_MATCH_{}", wild_name.to_ascii_uppercase()); // TODO: safer
        res.push(([wild_header, wild_wagi_header], wild_value.clone()));
    }

    Ok(res)
}

pub fn strip_forbidden_headers(req: &mut Request<Body>) {
    let headers = req.headers_mut();
    if let Some(host_header) = headers.get("Host") {
        if let Ok(host) = host_header.to_str() {
            if is_service_chaining_host(host) {
                headers.remove("Host");
            }
        }
    }
}

pub fn prepare_request_headers(
    req: &Request<Body>,
    route_match: &RouteMatch,
    client_addr: SocketAddr,
) -> Result<Vec<(String, String)>> {
    let mut res = Vec::new();
    for (name, value) in req
        .headers()
        .iter()
        .map(|(name, value)| (name.to_string(), std::str::from_utf8(value.as_bytes())))
    {
        let value = value?.to_string();
        res.push((name, value));
    }

    let default_host = http::HeaderValue::from_str("localhost")?;
    let host = std::str::from_utf8(
        req.headers()
            .get("host")
            .unwrap_or(&default_host)
            .as_bytes(),
    )?;

    // Set the environment information (path info, base path, etc) as headers.
    // In the future, we might want to have this information in a context
    // object as opposed to headers.
    for (keys, val) in compute_default_headers(req.uri(), host, route_match, client_addr)? {
        res.push((prepare_header_key(&keys[0]), val));
    }

    Ok(res)
}

pub fn append_headers(
    map: &mut http::HeaderMap,
    headers: Option<Vec<(String, String)>>,
) -> Result<()> {
    if let Some(src) = headers {
        for (k, v) in src.iter() {
            map.insert(
                http::header::HeaderName::from_str(k)?,
                http::header::HeaderValue::from_str(v)?,
            );
        }
    };

    Ok(())
}

fn prepare_header_key(key: &str) -> String {
    key.replace('_', "-").to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spin_header_keys() {
        assert_eq!(
            prepare_header_key("SPIN_FULL_URL"),
            "spin-full-url".to_string()
        );
        assert_eq!(
            prepare_header_key("SPIN_PATH_INFO"),
            "spin-path-info".to_string()
        );
        assert_eq!(
            prepare_header_key("SPIN_RAW_COMPONENT_ROUTE"),
            "spin-raw-component-route".to_string()
        );
    }
}
