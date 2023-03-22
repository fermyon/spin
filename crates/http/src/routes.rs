//! Route matching for the HTTP trigger.

#![deny(missing_docs)]

use anyhow::{anyhow, Result};
use http::Uri;
use indexmap::IndexMap;
use std::{borrow::Cow, fmt};

/// Router for the HTTP trigger.
#[derive(Clone, Debug)]
pub struct Router {
    /// Ordered map between a path and the component ID that should handle it.
    pub(crate) routes: IndexMap<RoutePattern, String>,
}

/// A detected duplicate route.
pub struct DuplicateRoute {
    /// The duplicated route pattern.
    pub route: RoutePattern,
    /// The raw route that was duplicated.
    pub replaced_id: String,
    /// The component ID corresponding to the duplicated route.
    pub effective_id: String,
}

impl Router {
    /// Builds a router based on application configuration.
    pub fn build<'a>(
        base: &str,
        component_routes: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<(Self, Vec<DuplicateRoute>)> {
        let mut routes = IndexMap::new();
        let mut duplicates = vec![];

        let routes_iter = component_routes.into_iter().map(|(component_id, route)| {
            (RoutePattern::from(base, route), component_id.to_string())
        });

        for (route, component_id) in routes_iter {
            let replaced = routes.insert(route.clone(), component_id.clone());
            if let Some(replaced) = replaced {
                duplicates.push(DuplicateRoute {
                    route: route.clone(),
                    replaced_id: replaced,
                    effective_id: component_id.clone(),
                });
            }
        }

        Ok((Self { routes }, duplicates))
    }

    /// Returns the constructed routes.
    pub fn routes(&self) -> impl Iterator<Item = (&RoutePattern, &String)> {
        self.routes.iter()
    }

    /// This returns the component id and route pattern for a matched route.
    pub fn route_full(&self, p: &str) -> Result<(&str, &RoutePattern)> {
        let matches = self.routes.iter().filter(|(rp, _)| rp.matches(p));

        let mut best_match: (Option<&str>, Option<&RoutePattern>, usize) = (None, None, 0); // matched id, pattern and length

        for (rp, id) in matches {
            match rp {
                RoutePattern::Exact(_m) => {
                    // Exact matching routes take precedence over wildcard matches.
                    return Ok((id, rp));
                }
                RoutePattern::Wildcard(m) => {
                    // Check and find longest matching prefix of wildcard pattern.
                    let (_id_opt, _rp_opt, len) = best_match;
                    if m.len() >= len {
                        best_match = (Some(id), Some(rp), m.len());
                    }
                }
            }
        }

        let (id, rp, _) = best_match;
        id.zip(rp)
            .ok_or_else(|| anyhow!("Cannot match route for path {p}"))
    }

    /// This returns the component ID that should handle the given path, or an error
    /// if no component matches.
    ///
    /// If multiple components could potentially handle the same request based on their
    /// defined routes, components with matching exact routes take precedence followed
    /// by matching wildcard patterns with the longest matching prefix.
    pub fn route(&self, p: &str) -> Result<&str> {
        self.route_full(p).map(|(r, _)| r)
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
    pub fn from<S: Into<String>>(base: S, path: S) -> Self {
        let path = Self::sanitize_with_base(base, path);
        match path.strip_suffix("/...") {
            Some(p) => Self::Wildcard(p.to_owned()),
            None => Self::Exact(path),
        }
    }

    /// Returns true if the given path fragment can be handled
    /// by the route pattern.
    pub fn matches<S: Into<String>>(&self, p: S) -> bool {
        let p = Self::sanitize(p);
        match self {
            RoutePattern::Exact(path) => &p == path,
            RoutePattern::Wildcard(pattern) => {
                &p == pattern || p.starts_with(&format!("{}/", pattern))
            }
        }
    }

    /// Resolves a relative path from the end of the matched path to the end of the string.
    pub fn relative(&self, uri: &str) -> Result<String> {
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
    pub fn path_or_prefix(&self) -> &str {
        match self {
            RoutePattern::Exact(s) => s,
            RoutePattern::Wildcard(s) => s,
        }
    }

    /// The full pattern, with trailing "/..." for Wildcard.
    pub fn full_pattern(&self) -> Cow<str> {
        match self {
            Self::Exact(path) => path.into(),
            Self::Wildcard(prefix) => format!("{}/...", prefix).into(),
        }
    }

    /// The full pattern, with trailing "/..." for Wildcard and "/" for root.
    pub fn full_pattern_non_empty(&self) -> Cow<str> {
        match self {
            Self::Exact(path) if path.is_empty() => "/".into(),
            _ => self.full_pattern(),
        }
    }

    /// Sanitizes the base and path and return a formed path.
    pub fn sanitize_with_base<S: Into<String>>(base: S, path: S) -> String {
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
    use spin_testing::init_tracing;

    use super::*;

    #[test]
    fn test_exact_route() {
        init_tracing();

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

        assert_eq!(
            r.route("/one/two/three/four")?,
            "onetwothree_wildcard".to_string()
        );

        // Test routing rule "exact beats wildcard" ...
        let mut routes = IndexMap::new();

        routes.insert(RoutePattern::from("/", "/one"), "one_exact".to_string());

        routes.insert(RoutePattern::from("/", "/..."), "wildcard".to_string());

        let r = Router { routes };

        assert_eq!(r.route("/one")?, "one_exact".to_string(),);

        Ok(())
    }

    #[test]
    fn sensible_routes_are_reachable() {
        let (routes, duplicates) = Router::build(
            "/",
            vec![
                ("/", "/"),
                ("/foo", "/foo"),
                ("/bar", "/bar"),
                ("/whee/...", "/whee/..."),
            ],
        )
        .unwrap();

        assert_eq!(4, routes.routes.len());
        assert_eq!(0, duplicates.len());
    }

    #[test]
    fn order_of_reachable_routes_is_preserved() {
        let (routes, _) = Router::build(
            "/",
            vec![
                ("/", "/"),
                ("/foo", "/foo"),
                ("/bar", "/bar"),
                ("/whee/...", "/whee/..."),
            ],
        )
        .unwrap();

        assert_eq!("/", routes.routes[0]);
        assert_eq!("/foo", routes.routes[1]);
        assert_eq!("/bar", routes.routes[2]);
        assert_eq!("/whee/...", routes.routes[3]);
    }

    #[test]
    fn duplicate_routes_are_unreachable() {
        let (routes, duplicates) = Router::build(
            "/",
            vec![
                ("/", "/"),
                ("first /foo", "/foo"),
                ("second /foo", "/foo"),
                ("/whee/...", "/whee/..."),
            ],
        )
        .unwrap();

        assert_eq!(3, routes.routes.len());
        assert_eq!(1, duplicates.len());
    }

    #[test]
    fn duplicate_routes_last_one_wins() {
        let (routes, duplicates) = Router::build(
            "/",
            vec![
                ("/", "/"),
                ("first /foo", "/foo"),
                ("second /foo", "/foo"),
                ("/whee/...", "/whee/..."),
            ],
        )
        .unwrap();

        assert_eq!("second /foo", routes.routes[1]);
        assert_eq!("first /foo", duplicates[0].replaced_id);
        assert_eq!("second /foo", duplicates[0].effective_id);
    }
}
