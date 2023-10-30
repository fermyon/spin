use anyhow::Context;
use spin_locked_app::MetadataKey;
use url::Url;

pub const ALLOWED_HOSTS_KEY: MetadataKey<Option<Vec<String>>> =
    MetadataKey::new("allowed_outbound_hosts");

/// Checks address against allowed hosts
///
/// Emits several warnings
pub fn check_address(
    address: &str,
    scheme: &str,
    allowed_hosts: &Option<AllowedHosts>,
    default: bool,
) -> bool {
    let Ok(address) = Address::parse(address, Some(scheme)) else {
        terminal::warn!(
            "A component tried to make a request to an address that could not be parsed {address}.",
        );
        return false;
    };
    let is_allowed = if let Some(allowed_hosts) = allowed_hosts {
        allowed_hosts.allows(&address)
    } else {
        default
    };

    if !is_allowed {
        terminal::warn!("A component tried to make a request to non-allowed address '{address}'.");
        let (host, port) = (address.host(), address.port());
        let msg = if let Some(port) = port {
            format!("`allowed_outbound_hosts = [\"{host}:{port}\"]`")
        } else {
            format!("`allowed_outbound_hosts = [\"{host}:$PORT\"]` (where $PORT is the correct port number)")
        };
        eprintln!("To allow requests, add {msg} to the manifest component section.");
    }
    is_allowed
}

/// An address is a url-like string that contains a host, a port, and an optional scheme
#[derive(Debug)]
pub struct Address {
    inner: Url,
    original: String,
    has_scheme: bool,
}

impl Address {
    /// Try to parse the address.
    ///
    /// If the parsing fails, the address is prepended with the scheme and parsing
    /// is tried again.
    pub fn parse(url: &str, scheme: Option<&str>) -> anyhow::Result<Self> {
        let mut has_scheme = true;
        let mut parsed = match Url::parse(url) {
            Ok(url) if url.has_host() => Ok(url),
            first_try => {
                // Parsing with 'scheme' resolves the ambiguity between 'spin.fermyon.com:80' and 'unix:80'.
                // Technically according to the spec a valid url *must* contain a scheme. However,
                // we allow url-like address strings without schemes, and we interpret the first part as the host.
                let second_try = format!("{}://{url}", scheme.unwrap_or("scheme"))
                    .as_str()
                    .try_into()
                    .context("could not convert into a url");
                has_scheme = scheme.is_some();
                match (second_try, first_try.map_err(|e| e.into())) {
                    (Ok(u), _) => Ok(u),
                    // Return an error preferring the error from the first attempt if present
                    (_, Err(e)) | (Err(e), _) => Err(e),
                }
            }
        }?;

        if parsed.port_or_known_default().is_none() {
            let _ = parsed.set_port(well_known_port(parsed.scheme()));
        }

        Ok(Self {
            inner: parsed,
            has_scheme,
            original: url.to_owned(),
        })
    }

    fn scheme(&self) -> Option<&str> {
        self.has_scheme.then_some(self.inner.scheme())
    }

    fn host(&self) -> &str {
        self.inner.host_str().unwrap_or_default()
    }

    fn port(&self) -> Option<u16> {
        self.inner
            .port_or_known_default()
            .or_else(|| well_known_port(self.scheme()?))
    }

    fn validate_as_config(&self) -> anyhow::Result<()> {
        if !["", "/"].contains(&self.inner.path()) {
            anyhow::bail!("config '{}' contains a path", self);
        }
        if self.inner.query().is_some() {
            anyhow::bail!("config '{}' contains a query string", self);
        }
        // We require configs contain a port
        if self.inner.port().is_none() {
            anyhow::bail!("config '{}' did not contain port", self)
        }

        Ok(())
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.original)
    }
}

fn well_known_port(scheme: &str) -> Option<u16> {
    match scheme {
        "postgres" => Some(5432),
        "mysql" => Some(3306),
        "redis" => Some(6379),
        _ => None,
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum AllowedHosts {
    All,
    SpecificHosts(Vec<AllowedHost>),
}

impl AllowedHosts {
    pub fn parse<S: AsRef<str>>(hosts: &[S]) -> anyhow::Result<AllowedHosts> {
        if hosts.len() == 1 && hosts[0].as_ref() == "insecure:allow-all" {
            return Ok(Self::All);
        }
        let mut allowed = Vec::with_capacity(hosts.len());
        for host in hosts {
            allowed.push(AllowedHost::parse(host)?)
        }
        Ok(Self::SpecificHosts(allowed))
    }

    /// Determine if the supplied address is allowed
    pub fn allows(&self, address: &Address) -> bool {
        match self {
            AllowedHosts::All => true,
            AllowedHosts::SpecificHosts(hosts) => hosts.iter().any(|h| h.allows(address)),
        }
    }

    pub fn allows_relative_url(&self) -> bool {
        match self {
            AllowedHosts::All => true,
            AllowedHosts::SpecificHosts(hosts) => hosts.iter().any(|h| h.allows_relative()),
        }
    }
}

impl Default for AllowedHosts {
    fn default() -> Self {
        Self::SpecificHosts(Vec::new())
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum AllowedHost {
    SelfHost,
    Host(Host),
}

impl AllowedHost {
    fn parse<U: AsRef<str>>(url: U) -> anyhow::Result<Self> {
        let url = url.as_ref().trim();
        if url == "self" {
            return Ok(Self::SelfHost);
        }
        Ok(Self::Host(Host::parse(url)?))
    }

    fn allows(&self, address: &Address) -> bool {
        match self {
            AllowedHost::SelfHost => false,
            AllowedHost::Host(h) => h.allows(address),
        }
    }

    pub fn allows_relative(&self) -> bool {
        matches!(self, Self::SelfHost)
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Host {
    scheme: Option<String>,
    host: String,
    port: u16,
}

impl Host {
    fn parse<U: AsRef<str>>(url: U) -> anyhow::Result<Self> {
        let address = Address::parse(url.as_ref(), None)?;
        address.validate_as_config()?;

        Ok(Self {
            scheme: address.scheme().map(ToOwned::to_owned),
            host: address.host().to_owned(),
            // `unwrap` is safe as `validate_as_config` checks that it is `Some`
            port: address.port().unwrap(),
        })
    }

    fn allows(&self, address: &Address) -> bool {
        let scheme_matches = self
            .scheme
            .as_deref()
            .map(|s| Some(s) == address.scheme())
            .unwrap_or(true);
        let host_matches = address.host() == self.host;
        let port_matches = address.port() == Some(self.port);

        println!("{scheme_matches:?} {host_matches:?} {port_matches:?}");
        scheme_matches && host_matches && port_matches
    }
}

#[cfg(test)]
mod test {
    impl AllowedHost {
        fn new(scheme: Option<&str>, host: impl Into<String>, port: u16) -> Self {
            Self::Host(Host {
                scheme: scheme.map(Into::into),
                host: host.into(),
                port,
            })
        }
    }

    use super::*;

    #[test]
    fn test_allowed_hosts_rejects_url_without_port() {
        assert!(AllowedHost::parse("http://spin.fermyon.dev").is_err());
    }

    #[test]
    fn test_allowed_hosts_accepts_url_with_port() {
        assert_eq!(
            AllowedHost::new(Some("http"), "spin.fermyon.dev", 4444),
            AllowedHost::parse("http://spin.fermyon.dev:4444").unwrap()
        );
        assert_eq!(
            AllowedHost::new(Some("http"), "spin.fermyon.dev", 4444),
            AllowedHost::parse("http://spin.fermyon.dev:4444/").unwrap()
        );
        assert_eq!(
            AllowedHost::new(Some("https"), "spin.fermyon.dev", 5555),
            AllowedHost::parse("https://spin.fermyon.dev:5555").unwrap()
        );
    }

    #[test]
    fn test_allowed_hosts_does_not_accept_plain_host_without_port() {
        assert!(AllowedHost::parse("spin.fermyon.dev").is_err());
    }

    #[test]
    fn test_allowed_hosts_accepts_plain_host_with_port() {
        assert_eq!(
            AllowedHost::new(None, "spin.fermyon.dev", 7777),
            AllowedHost::parse("spin.fermyon.dev:7777").unwrap()
        )
    }

    #[test]
    fn test_allowed_hosts_accepts_self() {
        assert_eq!(AllowedHost::SelfHost, AllowedHost::parse("self").unwrap());
    }

    #[test]
    fn test_allowed_hosts_accepts_localhost_addresses() {
        assert!(AllowedHost::parse("localhost").is_err());
        assert!(AllowedHost::parse("http://localhost").is_err());
        assert_eq!(
            AllowedHost::new(None, "localhost", 3001),
            AllowedHost::parse("localhost:3001").unwrap()
        );
        assert_eq!(
            AllowedHost::new(Some("http"), "localhost", 3001),
            AllowedHost::parse("http://localhost:3001").unwrap()
        );
    }

    #[test]
    fn test_allowed_hosts_accepts_ip_addresses() {
        assert!(AllowedHost::parse("http://192.168.1.1").is_err());
        assert_eq!(
            AllowedHost::new(Some("http"), "192.168.1.1", 3002),
            AllowedHost::parse("http://192.168.1.1:3002").unwrap()
        );
        assert_eq!(
            AllowedHost::new(None, "192.168.1.1", 3002),
            AllowedHost::parse("192.168.1.1:3002").unwrap()
        );
        assert_eq!(
            AllowedHost::new(Some("http"), "[::1]", 8001),
            AllowedHost::parse("http://[::1]:8001").unwrap()
        );
    }

    #[test]
    fn test_allowed_hosts_rejects_path() {
        assert!(AllowedHost::parse("http://spin.fermyon.dev/a").is_err());
        assert!(AllowedHost::parse("http://spin.fermyon.dev:6666/a/b").is_err());
    }

    #[test]
    fn test_allowed_hosts_respects_allow_all() {
        assert_eq!(
            AllowedHosts::All,
            AllowedHosts::parse(&["insecure:allow-all"]).unwrap()
        );
        assert!(AllowedHosts::parse(&["spin.fermyon.dev", "insecure:allow-all"]).is_err());
    }

    #[test]
    fn test_allowed_hosts_can_be_specific() {
        let allowed =
            AllowedHosts::parse(&["spin.fermyon.dev:443", "http://example.com:8383"]).unwrap();
        assert!(allowed
            .allows(&Address::parse("http://example.com:8383/foo/bar", Some("http")).unwrap()));
        assert!(
            allowed.allows(&Address::parse("https://spin.fermyon.dev/", Some("https")).unwrap())
        );
        assert!(!allowed.allows(&Address::parse("http://example.com/", Some("http")).unwrap()));
        assert!(!allowed.allows(&Address::parse("http://google.com/", Some("http")).unwrap()));
        assert!(allowed.allows(&Address::parse("spin.fermyon.dev:443", Some("https")).unwrap()));
        assert!(allowed.allows(&Address::parse("example.com:8383", Some("http")).unwrap()));
    }
}
