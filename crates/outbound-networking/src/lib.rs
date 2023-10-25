use anyhow::Context;
use url::Url;

/// Try to parse the url that may or not include the provided scheme.
///
/// If the parsing fails, the url is appended with the scheme and parsing
/// is tried again.
pub fn parse_url_with_host(url: &str, scheme: &str) -> anyhow::Result<Url> {
    match Url::parse(url) {
        Ok(url) if url.has_host() => Ok(url),
        first_try => {
            let second_try = format!("{scheme}://{url}")
                .as_str()
                .try_into()
                .context("could not convert into a url");
            match (second_try, first_try.map_err(|e| e.into())) {
                (Ok(u), _) => Ok(u),
                // Return an error preferring the error from the first attempt if present
                (_, Err(e)) | (Err(e), _) => Err(e),
            }
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum AllowedHosts {
    All,
    SpecificHosts(Vec<AllowedHost>),
}

impl AllowedHosts {
    pub fn parse<S: AsRef<str>>(hosts: &[S]) -> anyhow::Result<AllowedHosts> {
        // TODO: do we support this?
        // if hosts.len() == 1 && hosts[0].as_ref() == "insecure:allow-all" {
        //     return Ok(Self::All);
        // }
        let mut allowed = Vec::with_capacity(hosts.len());
        for host in hosts {
            allowed.push(AllowedHost::parse(host)?)
        }
        Ok(Self::SpecificHosts(allowed))
    }

    pub fn allows<U: TryInto<Url>>(&self, url: U) -> bool {
        match self {
            AllowedHosts::All => true,
            AllowedHosts::SpecificHosts(hosts) => {
                let Ok(url) = url.try_into() else {
                    return false;
                };
                hosts.iter().any(|h| h.allows(&url))
            }
        }
    }
}

impl Default for AllowedHosts {
    fn default() -> Self {
        Self::SpecificHosts(Vec::new())
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct AllowedHost {
    scheme: Option<String>,
    host: String,
    port: u16,
}

impl AllowedHost {
    fn parse<U: AsRef<str>>(url: U) -> anyhow::Result<Self> {
        let url_str = url.as_ref();
        let url: anyhow::Result<Url> = url_str
            .try_into()
            .with_context(|| format!("could not convert {url_str:?} into a url"));
        let (url, has_scheme) = match url {
            Ok(url) if url.has_host() => (url, true),
            first_try => {
                // If the url doesn't successfully parse try again with an added scheme.
                // This resolves the ambiguity between 'spin.fermyon.com:80' and 'unix:80'.
                // Technically according to the spec a valid url *must* contain a scheme. However,
                // we allow url-like strings without schemes, and we interpret the first part as the host.
                let second_try = format!("scheme://{url_str}")
                    .as_str()
                    .try_into()
                    .context("could not convert into a url");
                match (second_try, first_try) {
                    (Ok(u), _) => (u, false),
                    // Return an error preferring the error from the first attempt if present
                    (_, Err(e)) | (Err(e), _) => return Err(e),
                }
            }
        };
        let host = url.host_str().context("the url has no host")?.to_owned();

        if !["", "/"].contains(&url.path()) {
            anyhow::bail!("url contains a path")
        }
        if url.query().is_some() {
            anyhow::bail!("url contains a query string")
        }
        Ok(Self {
            scheme: has_scheme.then(|| url.scheme().to_owned()),
            host,
            port: url
                .port_or_known_default()
                .context("url did not contain port")?,
        })
    }

    fn allows(&self, url: &Url) -> bool {
        let scheme_matches = self
            .scheme
            .as_ref()
            .map(|s| s == url.scheme())
            .unwrap_or(true);
        let host_matches = url.host_str().unwrap_or_default() == self.host;
        let port_matches = url.port_or_known_default().unwrap_or_default() == self.port;

        scheme_matches && host_matches && port_matches
    }
}

#[cfg(test)]
mod test {
    impl AllowedHost {
        fn new(scheme: Option<&str>, host: impl Into<String>, port: u16) -> Self {
            Self {
                scheme: scheme.map(Into::into),
                host: host.into(),
                port,
            }
        }
    }

    use super::*;

    #[test]
    fn test_allowed_hosts_accepts_http_url() {
        assert_eq!(
            AllowedHost::new(Some("http"), "spin.fermyon.dev", 80),
            AllowedHost::parse("http://spin.fermyon.dev").unwrap()
        );
        assert_eq!(
            AllowedHost::new(Some("http"), "spin.fermyon.dev", 80),
            AllowedHost::parse("http://spin.fermyon.dev/").unwrap()
        );
        assert_eq!(
            AllowedHost::new(Some("https"), "spin.fermyon.dev", 443),
            AllowedHost::parse("https://spin.fermyon.dev").unwrap()
        );
    }

    #[test]
    fn test_allowed_hosts_accepts_http_url_with_port() {
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

    // #[test]
    // fn test_allowed_hosts_accepts_self() {
    // TODO: do we support this?
    //     assert_eq!(
    //         AllowedHost::host("self"),
    //         parse_allowed_http_host("self").unwrap()
    //     );
    // }

    #[test]
    fn test_allowed_hosts_accepts_localhost_addresses() {
        assert!(AllowedHost::parse("localhost").is_err());
        assert_eq!(
            AllowedHost::new(Some("http"), "localhost", 80),
            AllowedHost::parse("http://localhost").unwrap()
        );
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
        assert_eq!(
            AllowedHost::new(Some("http"), "192.168.1.1", 80),
            AllowedHost::parse("http://192.168.1.1").unwrap()
        );
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
        // TODO: do we support this?
        // assert_eq!(
        //     AllowedHosts::All,
        //     AllowedHosts::parse(&["insecure:allow-all"]).unwrap()
        // );
        assert!(AllowedHosts::parse(&["insecure:allow-all"]).is_err());
        assert!(AllowedHosts::parse(&["spin.fermyon.dev", "insecure:allow-all"]).is_err());
    }

    #[test]
    fn test_allowed_hosts_can_be_specific() {
        let allowed =
            AllowedHosts::parse(&["spin.fermyon.dev:443", "http://example.com:8383"]).unwrap();
        assert!(allowed.allows(Url::parse("http://example.com:8383/foo/bar").unwrap()));
        assert!(allowed.allows(Url::parse("https://spin.fermyon.dev/").unwrap()));
        assert!(!allowed.allows(Url::parse("http://example.com/").unwrap()));
        assert!(!allowed.allows(Url::parse("http://google.com/").unwrap()));
    }
}
