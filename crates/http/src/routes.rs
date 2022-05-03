//! Route matching for the HTTP trigger.

#![deny(missing_docs)]

use anyhow::{Context, Result};
use http::Uri;
use indexmap::IndexMap;
use spin_manifest::{ComponentMap, HttpConfig};
use std::fmt;

/// Router for the HTTP trigger.
#[derive(Clone, Debug)]
pub struct Router {
    /// Ordered map between a path and the component ID that should handle it.
    pub(crate) routes: IndexMap<RoutePattern, String>,
}

impl Router {
    /// Builds a router based on application configuration.
    pub(crate) fn build(
        base: &str,
        component_http_configs: &ComponentMap<HttpConfig>,
    ) -> Result<Self> {
        let routes = component_http_configs
            .iter()
            .map(|(component_id, http_config)| {
                (
                    RoutePattern::from(base, &http_config.route),
                    component_id.to_string(),
                )
            })
            .collect();

        Ok(Self { routes })
    }

    // This assumes the order of the components in the manifest has been
    // preserved, so the routing algorithm, which takes the order into account,
    // is correct.
    /// Returns the component ID that should handle the given path, or an error
    /// if no component matches.
    /// If there are multiple possible components registered for the same route or
    /// wildcard, this returns the last entry in the component map.
    pub(crate) fn route(&self, p: &str) -> Result<&str> {
        self.routes
            .iter()
            .rfind(|(rp, _)| rp.matches(p))
            .map(|(_, c)| c.as_ref())
            .with_context(|| format!("Cannot match route for path {}", p))
    }
}

/// Route patterns for HTTP components.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum RoutePattern {
    Exact(String),
    Wildcard(String),
}

impl RoutePattern {
    /// Returns a RoutePattern given a path fragment.
    pub(crate) fn from<S: Into<String>>(base: S, path: S) -> Self {
        let path = format!(
            "{}{}",
            Self::sanitize(base.into()),
            Self::sanitize(path.into())
        );
        match path.strip_suffix("/...") {
            Some(p) => Self::Wildcard(p.to_owned()),
            None => Self::Exact(path),
        }
    }

    /// Returns true if the given path fragment can be handled
    /// by the route pattern.
    pub(crate) fn matches<S: Into<String>>(&self, p: S) -> bool {
        let p = Self::sanitize(p);
        match self {
            RoutePattern::Exact(path) => &p == path,
            RoutePattern::Wildcard(pattern) => {
                &p == pattern || p.starts_with(&format!("{}/", pattern))
            }
        }
    }

    /// Resolves a relative path from the end of the matched path to the end of the string.
    pub(crate) fn relative(&self, uri: &str) -> Result<String> {
        let base = match self {
            Self::Exact(path) => path,
            Self::Wildcard(prefix) => prefix,
        };
        Ok(uri
            .parse::<Uri>()?
            .path()
            .strip_prefix(base)
            .unwrap_or_default()
            .to_owned())
    }

    /// Sanitizes the base and path and return a formed path.
    pub(crate) fn sanitize_with_base<S: Into<String>>(base: S, path: S) -> String {
        format!(
            "{}{}",
            Self::sanitize(base.into()),
            Self::sanitize(path.into())
        )
    }

    /// Strips the trailing slash from a string.
    fn sanitize<S: Into<String>>(s: S) -> String {
        let s = s.into();
        // TODO
        // This only strips a single trailing slash.
        // Should we attempt to strip all trailing slashes?
        match s.strip_suffix('/') {
            Some(s) => s.into(),
            None => s,
        }
    }
}

impl fmt::Display for RoutePattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            RoutePattern::Exact(path) => write!(f, "{}", path),
            RoutePattern::Wildcard(pattern) => write!(f, "{} (wildcard)", pattern),
        }
    }
}

#[cfg(test)]
mod route_tests {
    use super::*;
    use crate::tests::init;

    #[test]
    fn test_exact_route() {
        init();

        let rp = RoutePattern::from("/", "/foo/bar");
        assert!(rp.matches("/foo/bar"));
        assert!(rp.matches("/foo/bar/"));
        assert!(!rp.matches("/foo"));
        assert!(!rp.matches("/foo/bar/thisshouldbefalse"));
        assert!(!rp.matches("/abc"));

        let rp = RoutePattern::from("/base", "/foo");
        assert!(rp.matches("/base/foo"));
        assert!(rp.matches("/base/foo/"));
        assert!(!rp.matches("/base/foo/bar"));
        assert!(!rp.matches("/thishouldbefalse"));

        let rp = RoutePattern::from("/base/", "/foo");
        assert!(rp.matches("/base/foo"));
        assert!(rp.matches("/base/foo/"));
        assert!(!rp.matches("/base/foo/bar"));
        assert!(!rp.matches("/thishouldbefalse"));

        let rp = RoutePattern::from("/base/", "/foo/");
        assert!(rp.matches("/base/foo"));
        assert!(rp.matches("/base/foo/"));
        assert!(!rp.matches("/base/foo/bar"));
        assert!(!rp.matches("/thishouldbefalse"));
    }

    #[test]
    fn test_pattern_route() {
        let rp = RoutePattern::from("/", "/...");
        assert!(rp.matches("/foo/bar/"));
        assert!(rp.matches("/foo"));
        assert!(rp.matches("/foo/bar/baz"));
        assert!(rp.matches("/this/should/really/match/everything/"));
        assert!(rp.matches("/"));

        let rp = RoutePattern::from("/", "/foo/...");
        assert!(rp.matches("/foo/bar/"));
        assert!(rp.matches("/foo"));
        assert!(rp.matches("/foo/bar/baz"));
        assert!(!rp.matches("/this/should/really/not/match/everything/"));
        assert!(!rp.matches("/"));

        let rp = RoutePattern::from("/base", "/...");
        assert!(rp.matches("/base/foo/bar/"));
        assert!(rp.matches("/base/foo"));
        assert!(rp.matches("/base/foo/bar/baz"));
        assert!(rp.matches("/base/this/should/really/match/everything/"));
        assert!(rp.matches("/base"));
    }

    #[test]
    fn test_relative() -> Result<()> {
        assert_eq!(
            RoutePattern::from("/", "/foo").relative("/foo/bar")?,
            "/bar".to_string()
        );
        assert_eq!(
            RoutePattern::from("/base", "/foo").relative("/base/foo/bar")?,
            "/bar".to_string()
        );

        assert_eq!(
            RoutePattern::from("/", "/foo").relative("/foo")?,
            "".to_string()
        );
        assert_eq!(
            RoutePattern::from("/base", "/foo").relative("/base/foo")?,
            "".to_string()
        );

        assert_eq!(
            RoutePattern::from("/", "/static/...").relative("/static/images/abc.png")?,
            "/images/abc.png".to_string()
        );
        assert_eq!(
            RoutePattern::from("/base", "/static/...").relative("/base/static/images/abc.png")?,
            "/images/abc.png".to_string()
        );

        assert_eq!(
            RoutePattern::from("/base", "/static/...")
                .relative("/base/static/images/abc.png?abc=def&foo=bar")?,
            "/images/abc.png".to_string()
        );

        Ok(())
    }

    #[test]
    fn test_router() -> Result<()> {
        let mut routes = IndexMap::new();

        routes.insert(RoutePattern::from("/", "/foo"), "foo".to_string());
        routes.insert(RoutePattern::from("/", "/foo/bar"), "foobar".to_string());

        let r = Router { routes };

        assert_eq!(r.route("/foo")?, "foo".to_string());
        assert_eq!(r.route("/foo/bar")?, "foobar".to_string());

        let mut routes = IndexMap::new();

        routes.insert(RoutePattern::from("/base", "/foo"), "foo".to_string());
        routes.insert(
            RoutePattern::from("/base", "/foo/bar"),
            "foobar".to_string(),
        );

        let r = Router { routes };

        assert_eq!(r.route("/base/foo")?, "foo".to_string());
        assert_eq!(r.route("/base/foo/bar")?, "foobar".to_string());

        let mut routes = IndexMap::new();

        routes.insert(RoutePattern::from("/", "/..."), "all".to_string());

        let r = Router { routes };

        assert_eq!(r.route("/foo/bar")?, "all".to_string());
        assert_eq!(r.route("/abc/")?, "all".to_string());
        assert_eq!(r.route("/")?, "all".to_string());
        assert_eq!(
            r.route("/this/should/be/captured?abc=def")?,
            "all".to_string()
        );

        let mut routes = IndexMap::new();

        routes.insert(
            RoutePattern::from("/", "/one/..."),
            "one_wildcard".to_string(),
        );
        routes.insert(
            RoutePattern::from("/", "/one/two/..."),
            "onetwo_wildcard".to_string(),
        );
        routes.insert(
            RoutePattern::from("/", "/one/two/three/..."),
            "onetwothree_wildcard".to_string(),
        );

        let r = Router { routes };

        assert_eq!(
            r.route("/one/two/three/four")?,
            "onetwothree_wildcard".to_string()
        );

        let mut routes = IndexMap::new();

        routes.insert(
            RoutePattern::from("/", "/one/two/three/..."),
            "onetwothree_wildcard".to_string(),
        );
        routes.insert(
            RoutePattern::from("/", "/one/two/..."),
            "onetwo_wildcard".to_string(),
        );
        routes.insert(
            RoutePattern::from("/", "/one/..."),
            "one_wildcard".to_string(),
        );

        let r = Router { routes };

        assert_eq!(r.route("/one/two/three/four")?, "one_wildcard".to_string());

        Ok(())
    }
}
