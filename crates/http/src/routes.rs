//! Route matching for the HTTP trigger.

use std::{borrow::Cow, fmt};

use anyhow::{Context, Result};
use http::Uri;
use indexmap::IndexMap;

/// Router for the HTTP trigger.
#[derive(Clone, Debug)]
pub struct Router {
    /// Ordered map between a path and the component ID that should handle it.
    pub(crate) routes: IndexMap<RoutePattern, String>,
}

impl Router {
    /// Builds a router based on application configuration.
    pub(crate) fn build(
        component_routes: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self> {
        let routes = component_routes
            .into_iter()
            .map(|(component_id, route)| (RoutePattern::from(route), component_id))
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
pub enum RoutePattern {
    /// A route pattern that only matches the exact path given.
    Exact(String),
    /// A route pattern that matches any path starting with the given string.
    Wildcard(String),
}

impl RoutePattern {
    /// Returns a RoutePattern given a path fragment.
    pub fn from(path: impl Into<String>) -> Self {
        let path = Self::sanitize(path);
        match path.strip_suffix("/...") {
            Some(p) => Self::Wildcard(p.to_owned()),
            None => Self::Exact(path.to_owned()),
        }
    }

    /// Build a RoutePattern from the give base (prefix) and path suffix.
    pub fn with_base(base: impl Into<String>, path: impl AsRef<str>) -> Self {
        Self::from(Self::sanitize(base) + path.as_ref())
    }

    /// Returns true if the given path fragment can be handled
    /// by the route pattern.
    pub(crate) fn matches(&self, p: impl Into<String>) -> bool {
        let p = Self::sanitize(p);
        match self {
            RoutePattern::Exact(path) => &p == path,
            RoutePattern::Wildcard(pattern) => {
                &p == pattern || p.starts_with(&format!("{}/", pattern))
            }
        }
    }

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

    /// The full path (for Exact) or prefix (for Wildcard).
    pub(crate) fn path_or_prefix(&self) -> &str {
        match self {
            RoutePattern::Exact(s) => s,
            RoutePattern::Wildcard(s) => s,
        }
    }

    /// The full pattern, with trailing "/..." for Wildcard.
    pub(crate) fn full_pattern(&self) -> Cow<str> {
        match self {
            Self::Exact(path) => path.into(),
            Self::Wildcard(prefix) => format!("{}/...", prefix).into(),
        }
    }

    /// Strips the trailing slash from a string.
    pub(crate) fn sanitize(path: impl Into<String>) -> String {
        let path = path.into();
        // TODO
        // This only strips a single trailing slash.
        // Should we attempt to strip all trailing slashes?
        match path.strip_suffix('/') {
            Some(s) => s.into(),
            None => path,
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

    #[test]
    fn test_exact_route() {
        let rp = RoutePattern::from("/foo/bar");
        assert!(rp.matches("/foo/bar"));
        assert!(rp.matches("/foo/bar/"));
        assert!(!rp.matches("/foo"));
        assert!(!rp.matches("/foo/bar/thisshouldbefalse"));
        assert!(!rp.matches("/abc"));

        let rp = RoutePattern::from("/foo/bar/");
        assert!(rp.matches("/foo/bar"));
        assert!(rp.matches("/foo/bar/"));
        assert!(!rp.matches("/foo/bar/baz"));
        assert!(!rp.matches("/thishouldbefalse"));
    }

    #[test]
    fn test_pattern_route() {
        let rp = RoutePattern::from("/...");
        assert!(rp.matches("/foo/bar/"));
        assert!(rp.matches("/foo"));
        assert!(rp.matches("/foo/bar/baz"));
        assert!(rp.matches("/this/should/really/match/everything/"));
        assert!(rp.matches("/"));

        let rp = RoutePattern::from("/foo/...");
        assert!(rp.matches("/foo/bar/"));
        assert!(rp.matches("/foo"));
        assert!(rp.matches("/foo/bar/baz"));
        assert!(!rp.matches("/this/should/really/not/match/everything/"));
        assert!(!rp.matches("/"));
    }

    #[test]
    fn test_relative() -> Result<()> {
        assert_eq!(
            RoutePattern::from("/foo").relative("/foo/bar")?,
            "/bar".to_string()
        );
        assert_eq!(RoutePattern::from("/foo").relative("/foo")?, "".to_string());
        assert_eq!(
            RoutePattern::from("/static/...").relative("/static/images/abc.png")?,
            "/images/abc.png".to_string()
        );
        assert_eq!(
            RoutePattern::from("/static/...").relative("/static/images/abc.png?abc=def&foo=bar")?,
            "/images/abc.png".to_string()
        );

        Ok(())
    }

    #[test]
    fn test_router() -> Result<()> {
        let mut routes = IndexMap::new();

        routes.insert(RoutePattern::from("/foo"), "foo".to_string());
        routes.insert(RoutePattern::from("/foo/bar"), "foobar".to_string());

        let r = Router { routes };

        assert_eq!(r.route("/foo")?, "foo".to_string());
        assert_eq!(r.route("/foo/bar")?, "foobar".to_string());

        let mut routes = IndexMap::new();

        routes.insert(RoutePattern::from("/..."), "all".to_string());

        let r = Router { routes };

        assert_eq!(r.route("/foo/bar")?, "all".to_string());
        assert_eq!(r.route("/abc/")?, "all".to_string());
        assert_eq!(r.route("/")?, "all".to_string());
        assert_eq!(
            r.route("/this/should/be/captured?abc=def")?,
            "all".to_string()
        );

        let mut routes = IndexMap::new();

        routes.insert(RoutePattern::from("/one/..."), "one_wildcard".to_string());
        routes.insert(
            RoutePattern::from("/one/two/..."),
            "onetwo_wildcard".to_string(),
        );
        routes.insert(
            RoutePattern::from("/one/two/three/..."),
            "onetwothree_wildcard".to_string(),
        );

        let r = Router { routes };

        assert_eq!(
            r.route("/one/two/three/four")?,
            "onetwothree_wildcard".to_string()
        );

        let mut routes = IndexMap::new();

        routes.insert(
            RoutePattern::from("/one/two/three/..."),
            "onetwothree_wildcard".to_string(),
        );
        routes.insert(
            RoutePattern::from("/one/two/..."),
            "onetwo_wildcard".to_string(),
        );
        routes.insert(RoutePattern::from("/one/..."), "one_wildcard".to_string());

        let r = Router { routes };

        assert_eq!(r.route("/one/two/three/four")?, "one_wildcard".to_string());

        Ok(())
    }
}
