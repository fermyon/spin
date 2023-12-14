use std::ops::Range;

use anyhow::{bail, Context};
use spin_locked_app::MetadataKey;

pub const ALLOWED_HOSTS_KEY: MetadataKey<Vec<String>> = MetadataKey::new("allowed_outbound_hosts");

/// Checks address against allowed hosts
///
/// Emits several warnings
pub fn check_url(url: &str, scheme: &str, allowed_hosts: &AllowedHostsConfig) -> bool {
    let Ok(url) = OutboundUrl::parse(url, scheme) else {
        terminal::warn!(
            "A component tried to make a request to an url that could not be parsed {url}.",
        );
        return false;
    };
    let is_allowed = allowed_hosts.allows(&url);

    if !is_allowed {
        terminal::warn!("A component tried to make a request to non-allowed url '{url}'.");
        let (scheme, host, port) = (url.scheme, url.host, url.port);
        let msg = if let Some(port) = port {
            format!("`allowed_outbound_hosts = [\"{scheme}://{host}:{port}\"]`")
        } else {
            format!("`allowed_outbound_hosts = [\"{scheme}://{host}:$PORT\"]` (where $PORT is the correct port number)")
        };
        eprintln!("To allow requests, add {msg} to the manifest component section.");
    }
    is_allowed
}

/// An address is a url-like string that contains a host, a port, and an optional scheme
#[derive(Eq, Debug, Clone)]
pub struct AllowedHostConfig {
    original: String,
    scheme: SchemeConfig,
    host: HostConfig,
    port: PortConfig,
}

impl AllowedHostConfig {
    /// Try to parse the address.
    ///
    /// If the parsing fails, the address is prepended with the scheme and parsing
    /// is tried again.
    pub fn parse(url: impl Into<String>) -> anyhow::Result<Self> {
        let original = url.into();
        let url = original.trim();
        let (scheme, rest) = url.split_once("://").with_context(|| {
            format!("{url:?} does not contain a scheme (e.g., 'http://' or '*://')")
        })?;
        let (host, rest) = rest.split_once(':').unwrap_or((rest, ""));
        let port = match rest.split_once('/') {
            Some((port, path)) => {
                if !path.is_empty() {
                    bail!("{url:?} has a path but is not allowed to");
                }
                port
            }
            None => rest,
        };

        Ok(Self {
            scheme: SchemeConfig::parse(scheme)?,
            host: HostConfig::parse(host)?,
            port: PortConfig::parse(port, scheme)?,
            original,
        })
    }

    pub fn scheme(&self) -> &SchemeConfig {
        &self.scheme
    }

    pub fn host(&self) -> &HostConfig {
        &self.host
    }

    pub fn port(&self) -> &PortConfig {
        &self.port
    }

    fn allows(&self, url: &OutboundUrl) -> bool {
        self.scheme.allows(&url.scheme)
            && self.host.allows(&url.host)
            && self.port.allows(url.port, &url.scheme)
    }

    fn allows_relative(&self, schemes: &[&str]) -> bool {
        schemes.iter().any(|s| self.scheme.allows(s)) && self.host.allows_relative()
    }
}

impl PartialEq for AllowedHostConfig {
    fn eq(&self, other: &Self) -> bool {
        self.scheme == other.scheme && self.host == other.host && self.port == other.port
    }
}

impl std::fmt::Display for AllowedHostConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.original)
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum SchemeConfig {
    Any,
    List(Vec<String>),
}

impl SchemeConfig {
    fn parse(scheme: &str) -> anyhow::Result<Self> {
        if scheme == "*" {
            return Ok(Self::Any);
        }

        if scheme.starts_with('{') {
            // TODO:
            bail!("scheme lists are not yet supported")
        }

        if scheme.chars().any(|c| !c.is_alphabetic()) {
            anyhow::bail!(" scheme {scheme:?} contains non alphabetic character");
        }

        Ok(Self::List(vec![scheme.into()]))
    }

    pub fn allows_any(&self) -> bool {
        matches!(self, Self::Any)
    }

    fn allows(&self, scheme: &str) -> bool {
        match self {
            SchemeConfig::Any => true,
            SchemeConfig::List(l) => l.iter().any(|s| s.as_str() == scheme),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum HostConfig {
    Any,
    ToSelf,
    List(Vec<String>),
    Cidr(ipnet::IpNet),
}

impl HostConfig {
    fn parse(mut host: &str) -> anyhow::Result<Self> {
        host = host.trim();
        if host == "*" {
            return Ok(Self::Any);
        }

        if host == "self" {
            return Ok(Self::ToSelf);
        }

        if host.starts_with('{') {
            bail!("host lists are not yet supported")
        }

        if let Ok(net) = host.parse::<ipnet::IpNet>() {
            return Ok(Self::Cidr(net));
        }

        if matches!(host.split('/').skip(1).next(), Some(path) if !path.is_empty()) {
            bail!("hosts must not contain paths");
        }

        Ok(Self::List(vec![host.into()]))
    }

    fn allows(&self, host: &str) -> bool {
        match self {
            HostConfig::Any => true,
            HostConfig::List(l) => l.iter().any(|h| h.as_str() == host),
            HostConfig::ToSelf => false,
            HostConfig::Cidr(c) => {
                let Ok(ip) = host.parse::<ipnet::IpNet>() else {
                    return false;
                };
                c.contains(&ip)
            }
        }
    }

    fn allows_relative(&self) -> bool {
        matches!(self, Self::Any | Self::ToSelf)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PortConfig {
    Any,
    List(Vec<IndividualPortConfig>),
}

impl PortConfig {
    fn parse(port: &str, scheme: &str) -> anyhow::Result<PortConfig> {
        if port.is_empty() {
            return well_known_port(scheme)
                .map(|p| PortConfig::List(vec![IndividualPortConfig::Port(p)]))
                .with_context(|| format!("no port was provided and the scheme {scheme:?} does not have a known default port number"));
        }
        if port == "*" {
            return Ok(PortConfig::Any);
        }

        if port.starts_with('{') {
            // TODO:
            bail!("port lists are not yet supported")
        }

        let port = IndividualPortConfig::parse(port)?;

        Ok(Self::List(vec![port]))
    }

    fn allows(&self, port: Option<u16>, scheme: &str) -> bool {
        match self {
            PortConfig::Any => true,
            PortConfig::List(l) => {
                let port = match port.or_else(|| well_known_port(scheme)) {
                    Some(p) => p,
                    None => return false,
                };
                l.iter().any(|p| p.allows(port))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum IndividualPortConfig {
    Port(u16),
    Range(Range<u16>),
}

impl IndividualPortConfig {
    fn parse(port: &str) -> anyhow::Result<Self> {
        if let Some((start, end)) = port.split_once("..") {
            let start = start
                .parse()
                .with_context(|| format!("port range {port:?} contains non-number"))?;
            let end = end
                .parse()
                .with_context(|| format!("port range {port:?} contains non-number"))?;
            return Ok(Self::Range(start..end));
        }
        Ok(Self::Port(port.parse().with_context(|| {
            format!("port {port:?} is not a number")
        })?))
    }

    fn allows(&self, port: u16) -> bool {
        match self {
            IndividualPortConfig::Port(p) => p == &port,
            IndividualPortConfig::Range(r) => r.contains(&port),
        }
    }
}

fn well_known_port(scheme: &str) -> Option<u16> {
    match scheme {
        "postgres" => Some(5432),
        "mysql" => Some(3306),
        "redis" => Some(6379),
        "http" => Some(80),
        "https" => Some(443),
        _ => None,
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum AllowedHostsConfig {
    All,
    SpecificHosts(Vec<AllowedHostConfig>),
}

impl AllowedHostsConfig {
    pub fn parse<S: AsRef<str>>(hosts: &[S]) -> anyhow::Result<AllowedHostsConfig> {
        if hosts.len() == 1 && hosts[0].as_ref() == "insecure:allow-all" {
            bail!("'insecure:allow-all' is not allowed - use '*://*:*' instead if you really want to allow all outbound traffic'")
        }
        let mut allowed = Vec::with_capacity(hosts.len());
        for host in hosts {
            allowed.push(AllowedHostConfig::parse(host.as_ref().to_owned())?)
        }
        Ok(Self::SpecificHosts(allowed))
    }

    /// Determine if the supplied url is allowed
    pub fn allows(&self, url: &OutboundUrl) -> bool {
        match self {
            AllowedHostsConfig::All => true,
            AllowedHostsConfig::SpecificHosts(hosts) => hosts.iter().any(|h| h.allows(url)),
        }
    }

    pub fn allows_relative_url(&self, schemes: &[&str]) -> bool {
        match self {
            AllowedHostsConfig::All => true,
            AllowedHostsConfig::SpecificHosts(hosts) => {
                hosts.iter().any(|h| h.allows_relative(schemes))
            }
        }
    }
}

impl Default for AllowedHostsConfig {
    fn default() -> Self {
        Self::SpecificHosts(Vec::new())
    }
}

#[derive(Debug, Clone)]
pub struct OutboundUrl {
    scheme: String,
    host: String,
    port: Option<u16>,
    original: String,
}

impl OutboundUrl {
    pub fn parse(url: impl Into<String>, scheme: &str) -> anyhow::Result<Self> {
        let mut url = url.into();
        let original = url.clone();

        // Ensure that the authority is url encoded. Since the authority is ignored after this,
        // we can always url encode the authority even if it is already encoded.
        if let Some(at) = url.find('@') {
            let scheme_end = url.find("://").map(|e| e + 3).unwrap_or(0);
            let userinfo = &url[scheme_end..at];

            let encoded = urlencoding::encode(userinfo);
            let prefix = &url[..scheme_end];
            let suffix = &url[scheme_end + userinfo.len()..];
            url = format!("{prefix}{encoded}{suffix}");
        }

        let parsed = match url::Url::parse(&url) {
            Ok(url) if url.has_host() => Ok(url),
            first_try => {
                let second_try: anyhow::Result<url::Url> = format!("{scheme}://{url}")
                    .as_str()
                    .try_into()
                    .context("could not convert into a url");
                match (second_try, first_try.map_err(|e| e.into())) {
                    (Ok(u), _) => Ok(u),
                    // Return an error preferring the error from the first attempt if present
                    (_, Err(e)) | (Err(e), _) => Err(e),
                }
            }
        }?;

        Ok(Self {
            scheme: parsed.scheme().to_owned(),
            host: parsed
                .host_str()
                .with_context(|| format!("{url:?} does not have a host component"))?
                .to_owned(),
            port: parsed.port(),
            original,
        })
    }
}

impl std::fmt::Display for OutboundUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.original)
    }
}

#[cfg(test)]
mod test {
    impl AllowedHostConfig {
        fn new(scheme: SchemeConfig, host: HostConfig, port: PortConfig) -> Self {
            Self {
                scheme,
                host,
                port,
                original: String::new(),
            }
        }
    }

    impl SchemeConfig {
        fn new(scheme: &str) -> Self {
            Self::List(vec![scheme.into()])
        }
    }

    impl HostConfig {
        fn new(host: &str) -> Self {
            Self::List(vec![host.into()])
        }
    }

    impl PortConfig {
        fn new(port: u16) -> Self {
            Self::List(vec![IndividualPortConfig::Port(port)])
        }

        fn range(port: Range<u16>) -> Self {
            Self::List(vec![IndividualPortConfig::Range(port)])
        }
    }

    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_allowed_hosts_accepts_url_with_port() {
        assert_eq!(
            AllowedHostConfig::new(
                SchemeConfig::new("http"),
                HostConfig::new("spin.fermyon.dev"),
                PortConfig::new(4444)
            ),
            AllowedHostConfig::parse("http://spin.fermyon.dev:4444").unwrap()
        );
        assert_eq!(
            AllowedHostConfig::new(
                SchemeConfig::new("http"),
                HostConfig::new("spin.fermyon.dev"),
                PortConfig::new(4444)
            ),
            AllowedHostConfig::parse("http://spin.fermyon.dev:4444/").unwrap()
        );
        assert_eq!(
            AllowedHostConfig::new(
                SchemeConfig::new("https"),
                HostConfig::new("spin.fermyon.dev"),
                PortConfig::new(5555)
            ),
            AllowedHostConfig::parse("https://spin.fermyon.dev:5555").unwrap()
        );
    }

    #[test]
    fn test_allowed_hosts_accepts_url_with_port_range() {
        assert_eq!(
            AllowedHostConfig::new(
                SchemeConfig::new("http"),
                HostConfig::new("spin.fermyon.dev"),
                PortConfig::range(4444..5555)
            ),
            AllowedHostConfig::parse("http://spin.fermyon.dev:4444..5555").unwrap()
        );
    }

    #[test]
    fn test_allowed_hosts_does_not_accept_plain_host_without_port() {
        assert!(AllowedHostConfig::parse("spin.fermyon.dev").is_err());
    }

    #[test]
    fn test_allowed_hosts_does_not_accept_plain_host_without_scheme() {
        assert!(AllowedHostConfig::parse("spin.fermyon.dev:80").is_err());
    }

    #[test]
    fn test_allowed_hosts_accepts_host_with_glob_scheme() {
        assert_eq!(
            AllowedHostConfig::new(
                SchemeConfig::Any,
                HostConfig::new("spin.fermyon.dev"),
                PortConfig::new(7777)
            ),
            AllowedHostConfig::parse("*://spin.fermyon.dev:7777").unwrap()
        )
    }

    #[test]
    fn test_allowed_hosts_accepts_self() {
        assert_eq!(
            AllowedHostConfig::new(
                SchemeConfig::new("http"),
                HostConfig::ToSelf,
                PortConfig::new(80)
            ),
            AllowedHostConfig::parse("http://self").unwrap()
        );
    }

    #[test]
    fn test_allowed_hosts_accepts_localhost_addresses() {
        assert!(AllowedHostConfig::parse("localhost").is_err());
        assert_eq!(
            AllowedHostConfig::new(
                SchemeConfig::new("http"),
                HostConfig::new("localhost"),
                PortConfig::new(80)
            ),
            AllowedHostConfig::parse("http://localhost").unwrap()
        );
        assert!(AllowedHostConfig::parse("localhost:3001").is_err());
        assert_eq!(
            AllowedHostConfig::new(
                SchemeConfig::new("http"),
                HostConfig::new("localhost"),
                PortConfig::new(3001)
            ),
            AllowedHostConfig::parse("http://localhost:3001").unwrap()
        );
    }

    #[test]
    fn test_allowed_hosts_accepts_ip_addresses() {
        assert_eq!(
            AllowedHostConfig::new(
                SchemeConfig::new("http"),
                HostConfig::new("192.168.1.1"),
                PortConfig::new(80)
            ),
            AllowedHostConfig::parse("http://192.168.1.1").unwrap()
        );
        assert_eq!(
            AllowedHostConfig::new(
                SchemeConfig::new("http"),
                HostConfig::new("192.168.1.1"),
                PortConfig::new(3002)
            ),
            AllowedHostConfig::parse("http://192.168.1.1:3002").unwrap()
        );
        // assert_eq!(
        //     AllowedHostConfig::new(Some("http"), "[::1]", 8001),
        //     AllowedHostConfig::parse("http://[::1]:8001").unwrap()
        // );
    }

    #[test]
    fn test_allowed_hosts_accepts_ip_cidr() {
        assert_eq!(
            AllowedHostConfig::new(
                SchemeConfig::Any,
                HostConfig::Cidr(ipnet::IpNet::V4(
                    ipnet::Ipv4Net::new(Ipv4Addr::new(127, 0, 0, 0), 24).unwrap()
                )),
                PortConfig::new(80)
            ),
            AllowedHostConfig::parse("*://127.0.0.0/24:80").unwrap()
        );
        assert!(AllowedHostConfig::parse("*://127.0.0.0/24").is_err());
    }

    #[test]
    fn test_allowed_hosts_rejects_path() {
        assert!(AllowedHostConfig::parse("http://spin.fermyon.dev/a").is_err());
        assert!(AllowedHostConfig::parse("http://spin.fermyon.dev:6666/a/b").is_err());
    }

    #[test]
    fn test_allowed_hosts_respects_allow_all() {
        assert!(AllowedHostsConfig::parse(&["insecure:allow-all"]).is_err());
        assert!(AllowedHostsConfig::parse(&["spin.fermyon.dev", "insecure:allow-all"]).is_err());
    }

    #[test]
    fn test_allowed_all_globs() {
        assert_eq!(
            AllowedHostConfig::new(SchemeConfig::Any, HostConfig::Any, PortConfig::Any),
            AllowedHostConfig::parse("*://*:*").unwrap()
        );
    }

    #[test]
    fn test_allowed_hosts_can_be_specific() {
        let allowed =
            AllowedHostsConfig::parse(&["*://spin.fermyon.dev:443", "http://example.com:8383"])
                .unwrap();
        assert!(
            allowed.allows(&OutboundUrl::parse("http://example.com:8383/foo/bar", "http").unwrap())
        );
        assert!(allowed.allows(&OutboundUrl::parse("https://spin.fermyon.dev/", "https").unwrap()));
        assert!(!allowed.allows(&OutboundUrl::parse("http://example.com/", "http").unwrap()));
        assert!(!allowed.allows(&OutboundUrl::parse("http://google.com/", "http").unwrap()));
        assert!(allowed.allows(&OutboundUrl::parse("spin.fermyon.dev:443", "https").unwrap()));
        assert!(allowed.allows(&OutboundUrl::parse("example.com:8383", "http").unwrap()));
    }

    #[test]
    fn test_hash_char_in_db_password() {
        let allowed = AllowedHostsConfig::parse(&["mysql://xyz.com"]).unwrap();
        assert!(
            allowed.allows(&OutboundUrl::parse("mysql://user:pass#word@xyz.com", "mysql").unwrap())
        );
        assert!(allowed
            .allows(&OutboundUrl::parse("mysql://user%3Apass%23word@xyz.com", "mysql").unwrap()));
        assert!(allowed.allows(&OutboundUrl::parse("user%3Apass%23word@xyz.com", "mysql").unwrap()));
    }
}
