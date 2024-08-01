//! Implementation for the Spin HTTP engine.

mod headers;
mod instrument;
mod server;
mod spin;
mod tls;
mod wagi;
mod wasi;

use std::{
    collections::HashMap,
    error::Error,
    net::{Ipv4Addr, SocketAddr, ToSocketAddrs},
    path::PathBuf,
    sync::Arc,
};

use anyhow::{bail, Context};
use clap::Args;
use serde::Deserialize;
use spin_app::App;
use spin_http::{config::HttpTriggerConfig, routes::Router};
use spin_trigger2::Trigger;
use tokio::net::TcpListener;
use wasmtime_wasi_http::bindings::wasi::http::types::ErrorCode;

use server::HttpServer;

pub use tls::TlsConfig;

pub(crate) use wasmtime_wasi_http::body::HyperIncomingBody as Body;

pub(crate) type TriggerApp = spin_trigger2::TriggerApp<HttpTrigger>;
pub(crate) type TriggerInstanceBuilder<'a> = spin_trigger2::TriggerInstanceBuilder<'a, HttpTrigger>;

#[derive(Args)]
pub struct CliArgs {
    /// IP address and port to listen on
    #[clap(long = "listen", env = "SPIN_HTTP_LISTEN_ADDR", default_value = "127.0.0.1:3000", value_parser = parse_listen_addr)]
    pub address: SocketAddr,

    /// The path to the certificate to use for https, if this is not set, normal http will be used. The cert should be in PEM format
    #[clap(long, env = "SPIN_TLS_CERT", requires = "tls-key")]
    pub tls_cert: Option<PathBuf>,

    /// The path to the certificate key to use for https, if this is not set, normal http will be used. The key should be in PKCS#8 format
    #[clap(long, env = "SPIN_TLS_KEY", requires = "tls-cert")]
    pub tls_key: Option<PathBuf>,
}

impl CliArgs {
    fn into_tls_config(self) -> Option<TlsConfig> {
        match (self.tls_cert, self.tls_key) {
            (Some(cert_path), Some(key_path)) => Some(TlsConfig {
                cert_path,
                key_path,
            }),
            (None, None) => None,
            _ => unreachable!(),
        }
    }
}

pub(crate) type InstanceState = ();

/// The Spin HTTP trigger.
pub struct HttpTrigger {
    listen_addr: SocketAddr,
    tls_config: Option<TlsConfig>,
    router: Router,
    // Component ID -> component trigger config
    component_trigger_configs: HashMap<String, HttpTriggerConfig>,
}

impl Trigger for HttpTrigger {
    const TYPE: &'static str = "http";

    type CliArgs = CliArgs;
    type InstanceState = InstanceState;

    fn new(cli_args: Self::CliArgs, app: &spin_app::App) -> anyhow::Result<Self> {
        Self::validate_app(app)?;

        let component_trigger_configs = HashMap::from_iter(
            app.trigger_configs::<HttpTriggerConfig>("http")?
                .into_iter()
                .map(|(_, config)| (config.component.clone(), config)),
        );

        let component_routes = component_trigger_configs
            .iter()
            .map(|(component_id, config)| (component_id.as_str(), &config.route));
        let (router, duplicate_routes) = Router::build("/", component_routes)?;
        if !duplicate_routes.is_empty() {
            tracing::error!(
                "The following component routes are duplicates and will never be used:"
            );
            for dup in &duplicate_routes {
                tracing::error!(
                    "  {}: {} (duplicate of {})",
                    dup.replaced_id,
                    dup.route(),
                    dup.effective_id,
                );
            }
        }
        tracing::trace!(
            "Constructed router: {:?}",
            router.routes().collect::<Vec<_>>()
        );

        Ok(Self {
            listen_addr: cli_args.address,
            tls_config: cli_args.into_tls_config(),
            router,
            component_trigger_configs,
        })
    }

    async fn run(self, trigger_app: TriggerApp) -> anyhow::Result<()> {
        let Self {
            listen_addr,
            tls_config,
            router,
            component_trigger_configs,
        } = self;

        let listener = TcpListener::bind(listen_addr)
            .await
            .with_context(|| format!("Unable to listen on {listen_addr}"))?;

        let server = Arc::new(HttpServer::new(
            listen_addr,
            trigger_app,
            router,
            component_trigger_configs,
        )?);

        if let Some(tls_config) = tls_config {
            server.serve_tls(listener, tls_config).await?
        } else {
            server.serve(listener).await?
        };

        Ok(())
    }

    fn supported_host_requirements() -> Vec<&'static str> {
        vec![spin_app::locked::SERVICE_CHAINING_KEY]
    }
}

impl HttpTrigger {
    fn validate_app(app: &App) -> anyhow::Result<()> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct TriggerMetadata {
            base: Option<String>,
        }
        if let Some(TriggerMetadata { base: Some(base) }) = app.get_trigger_metadata("http")? {
            if base == "/" {
                tracing::warn!("This application has the deprecated trigger 'base' set to the default value '/'. This may be an error in the future!");
            } else {
                bail!("This application is using the deprecated trigger 'base' field. The base must be prepended to each [[trigger.http]]'s 'route'.")
            }
        }
        Ok(())
    }
}

fn parse_listen_addr(addr: &str) -> anyhow::Result<SocketAddr> {
    let addrs: Vec<SocketAddr> = addr.to_socket_addrs()?.collect();
    // Prefer 127.0.0.1 over e.g. [::1] because CHANGE IS HARD
    if let Some(addr) = addrs
        .iter()
        .find(|addr| addr.is_ipv4() && addr.ip() == Ipv4Addr::LOCALHOST)
    {
        return Ok(*addr);
    }
    // Otherwise, take the first addr (OS preference)
    addrs.into_iter().next().context("couldn't resolve address")
}

#[derive(Debug, PartialEq)]
enum NotFoundRouteKind {
    Normal(String),
    WellKnown,
}

/// Translate a [`hyper::Error`] to a wasi-http `ErrorCode` in the context of a request.
pub fn hyper_request_error(err: hyper::Error) -> ErrorCode {
    // If there's a source, we might be able to extract a wasi-http error from it.
    if let Some(cause) = err.source() {
        if let Some(err) = cause.downcast_ref::<ErrorCode>() {
            return err.clone();
        }
    }

    tracing::warn!("hyper request error: {err:?}");

    ErrorCode::HttpProtocolError
}

pub fn dns_error(rcode: String, info_code: u16) -> ErrorCode {
    ErrorCode::DnsError(wasmtime_wasi_http::bindings::http::types::DnsErrorPayload {
        rcode: Some(rcode),
        info_code: Some(info_code),
    })
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use http::Request;

    use super::{headers::*, *};

    #[test]
    fn test_default_headers() -> Result<()> {
        let scheme = "https";
        let host = "fermyon.dev";
        let trigger_route = "/foo/...";
        let component_path = "/foo";
        let path_info = "/bar";
        let client_addr: SocketAddr = "127.0.0.1:8777".parse().unwrap();

        let req_uri = format!(
            "{}://{}{}{}?key1=value1&key2=value2",
            scheme, host, component_path, path_info
        );

        let req = http::Request::builder()
            .method("POST")
            .uri(req_uri)
            .body("")?;

        let (router, _) = Router::build("/", [("DUMMY", &trigger_route.into())])?;
        let route_match = router.route("/foo/bar")?;

        let default_headers = compute_default_headers(req.uri(), host, &route_match, client_addr)?;

        assert_eq!(
            search(&FULL_URL, &default_headers).unwrap(),
            "https://fermyon.dev/foo/bar?key1=value1&key2=value2".to_string()
        );
        assert_eq!(
            search(&PATH_INFO, &default_headers).unwrap(),
            "/bar".to_string()
        );
        assert_eq!(
            search(&MATCHED_ROUTE, &default_headers).unwrap(),
            "/foo/...".to_string()
        );
        assert_eq!(
            search(&BASE_PATH, &default_headers).unwrap(),
            "/".to_string()
        );
        assert_eq!(
            search(&RAW_COMPONENT_ROUTE, &default_headers).unwrap(),
            "/foo/...".to_string()
        );
        assert_eq!(
            search(&COMPONENT_ROUTE, &default_headers).unwrap(),
            "/foo".to_string()
        );
        assert_eq!(
            search(&CLIENT_ADDR, &default_headers).unwrap(),
            "127.0.0.1:8777".to_string()
        );

        Ok(())
    }

    #[test]
    fn test_default_headers_with_named_wildcards() -> Result<()> {
        let scheme = "https";
        let host = "fermyon.dev";
        let trigger_route = "/foo/:userid/...";
        let component_path = "/foo";
        let path_info = "/bar";
        let client_addr: SocketAddr = "127.0.0.1:8777".parse().unwrap();

        let req_uri = format!(
            "{}://{}{}/42{}?key1=value1&key2=value2",
            scheme, host, component_path, path_info
        );

        let req = http::Request::builder()
            .method("POST")
            .uri(req_uri)
            .body("")?;

        let (router, _) = Router::build("/", [("DUMMY", &trigger_route.into())])?;
        let route_match = router.route("/foo/42/bar")?;

        let default_headers = compute_default_headers(req.uri(), host, &route_match, client_addr)?;

        // TODO: we currently replace the scheme with HTTP. When TLS is supported, this should be fixed.
        assert_eq!(
            search(&FULL_URL, &default_headers).unwrap(),
            "https://fermyon.dev/foo/42/bar?key1=value1&key2=value2".to_string()
        );
        assert_eq!(
            search(&PATH_INFO, &default_headers).unwrap(),
            "/bar".to_string()
        );
        assert_eq!(
            search(&MATCHED_ROUTE, &default_headers).unwrap(),
            "/foo/:userid/...".to_string()
        );
        assert_eq!(
            search(&BASE_PATH, &default_headers).unwrap(),
            "/".to_string()
        );
        assert_eq!(
            search(&RAW_COMPONENT_ROUTE, &default_headers).unwrap(),
            "/foo/:userid/...".to_string()
        );
        assert_eq!(
            search(&COMPONENT_ROUTE, &default_headers).unwrap(),
            "/foo/:userid".to_string()
        );
        assert_eq!(
            search(&CLIENT_ADDR, &default_headers).unwrap(),
            "127.0.0.1:8777".to_string()
        );

        assert_eq!(
            search(
                &["SPIN_PATH_MATCH_USERID", "X_PATH_MATCH_USERID"],
                &default_headers
            )
            .unwrap(),
            "42".to_string()
        );

        Ok(())
    }

    fn search(keys: &[&str; 2], headers: &[([String; 2], String)]) -> Option<String> {
        let mut res: Option<String> = None;
        for (k, v) in headers {
            if k[0] == keys[0] && k[1] == keys[1] {
                res = Some(v.clone());
            }
        }

        res
    }

    #[test]
    fn parse_listen_addr_prefers_ipv4() {
        let addr = parse_listen_addr("localhost:12345").unwrap();
        assert_eq!(addr.ip(), Ipv4Addr::LOCALHOST);
        assert_eq!(addr.port(), 12345);
    }

    #[test]
    fn forbidden_headers_are_removed() {
        let mut req = Request::get("http://test.spin.internal")
            .header("Host", "test.spin.internal")
            .header("accept", "text/plain")
            .body(Default::default())
            .unwrap();

        strip_forbidden_headers(&mut req);

        assert_eq!(1, req.headers().len());
        assert!(req.headers().get("Host").is_none());

        let mut req = Request::get("http://test.spin.internal")
            .header("Host", "test.spin.internal:1234")
            .header("accept", "text/plain")
            .body(Default::default())
            .unwrap();

        strip_forbidden_headers(&mut req);

        assert_eq!(1, req.headers().len());
        assert!(req.headers().get("Host").is_none());
    }

    #[test]
    fn non_forbidden_headers_are_not_removed() {
        let mut req = Request::get("http://test.example.com")
            .header("Host", "test.example.org")
            .header("accept", "text/plain")
            .body(Default::default())
            .unwrap();

        strip_forbidden_headers(&mut req);

        assert_eq!(2, req.headers().len());
        assert!(req.headers().get("Host").is_some());
    }
}
