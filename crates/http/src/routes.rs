//! Route matching for the HTTP trigger.

#![deny(missing_docs)]

use anyhow::{bail, Result};
use http::Uri;
use indexmap::IndexMap;
use spin_config::{ApplicationTrigger, Configuration, CoreComponent};
use std::fmt::Debug;
use tracing::log;

// TODO
// The current implementation of the router clones the components, which could
// become costly if we have a lot of components.
// The router should borrow the components, which needs to introduce a lifetime
// parameter which surfaces in the HTTP trigger (and which needs a &'static because
// of the Hyper server.)
//
// For now we continue to use the router using owned data, but in the future it might
// make sense to try borrowing the components from the trigger.

/// Router for the HTTP trigger.
#[derive(Clone, Debug)]
pub(crate) struct Router {
    /// Ordered map between a path and the component that should handle it.
    pub(crate) routes: IndexMap<RoutePattern, CoreComponent>,
}

impl Router {
    /// Builds a router based on application configuration.
    pub(crate) fn build(app: &Configuration<CoreComponent>) -> Result<Self> {
        let ApplicationTrigger::Http(app_trigger) = app.info.trigger.clone();
        let routes = app
            .components
            .iter()
            .map(|c| {
                let spin_config::TriggerConfig::Http(trigger) = &c.trigger;
                (
                    RoutePattern::from(&app_trigger.base, &trigger.route),
                    c.clone(),
                )
            })
            .collect();

        log::trace!(
            "Constructed router for application {}: {:?}",
            app.info.name,
            routes
        );

        Ok(Self { routes })
    }

    // This assumes the order of the components in the app configuration vector
    // has been preserved, so the routing algorithm, which takes the order into
    // account, is correct. This seems to be the case with the TOML deserializer,
    // but might not hold if the application configuration is deserialized in
    // other ways.

    /// Returns the component that should handle the given path, or an error
    /// if no component matches.
    /// If there are multiple possible components registered for the same route or
    /// wildcard, this returns the last one in the components vector.
    pub(crate) fn route<S: Into<String> + Debug>(&self, p: S) -> Result<CoreComponent> {
        let p = p.into();

        let matches = &self
            .routes
            .iter()
            .filter(|(rp, _)| rp.matches(&p))
            .map(|(_, c)| c)
            .collect::<Vec<&CoreComponent>>();

        match matches.last() {
            Some(c) => Ok((*c).clone()),
            None => bail!("Cannot match route for path {}", p),
        }
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

#[cfg(test)]
mod route_tests {
    use std::collections::HashMap;

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

        let foo_component = named_component("foo");
        let foobar_component = named_component("foobar");

        routes.insert(RoutePattern::from("/", "/foo"), foo_component);
        routes.insert(RoutePattern::from("/", "/foo/bar"), foobar_component);

        let r = Router { routes };

        assert_eq!(r.route("/foo")?.id, "foo".to_string());
        assert_eq!(r.route("/foo/bar")?.id, "foobar".to_string());

        let mut routes = IndexMap::new();

        let foo_component = named_component("foo");
        let foobar_component = named_component("foobar");

        routes.insert(RoutePattern::from("/base", "/foo"), foo_component);
        routes.insert(RoutePattern::from("/base", "/foo/bar"), foobar_component);

        let r = Router { routes };

        assert_eq!(r.route("/base/foo")?.id, "foo".to_string());
        assert_eq!(r.route("/base/foo/bar")?.id, "foobar".to_string());

        let mut routes = IndexMap::new();

        let all_component = named_component("all");
        routes.insert(RoutePattern::from("/", "/..."), all_component);

        let r = Router { routes };

        assert_eq!(r.route("/foo/bar")?.id, "all".to_string());
        assert_eq!(r.route("/abc/")?.id, "all".to_string());
        assert_eq!(r.route("/")?.id, "all".to_string());
        assert_eq!(
            r.route("/this/should/be/captured?abc=def")?.id,
            "all".to_string()
        );

        let mut routes = IndexMap::new();

        let one_wildcard = named_component("one_wildcard");
        let onetwo_wildcard = named_component("onetwo_wildcard");
        let onetwothree_wildcard = named_component("onetwothree_wildcard");

        routes.insert(RoutePattern::from("/", "/one/..."), one_wildcard);
        routes.insert(RoutePattern::from("/", "/one/two/..."), onetwo_wildcard);
        routes.insert(
            RoutePattern::from("/", "/one/two/three/..."),
            onetwothree_wildcard,
        );

        let r = Router { routes };

        assert_eq!(
            r.route("/one/two/three/four")?.id,
            "onetwothree_wildcard".to_string()
        );

        let mut routes = IndexMap::new();

        let one_wildcard = named_component("one_wildcard");
        let onetwo_wildcard = named_component("onetwo_wildcard");
        let onetwothree_wildcard = named_component("onetwothree_wildcard");

        routes.insert(
            RoutePattern::from("/", "/one/two/three/..."),
            onetwothree_wildcard,
        );
        routes.insert(RoutePattern::from("/", "/one/two/..."), onetwo_wildcard);
        routes.insert(RoutePattern::from("/", "/one/..."), one_wildcard);

        let r = Router { routes };

        assert_eq!(
            r.route("/one/two/three/four")?.id,
            "one_wildcard".to_string()
        );

        Ok(())
    }

    fn named_component(id: &str) -> CoreComponent {
        CoreComponent {
            id: id.to_string(),
            source: spin_config::ModuleSource::FileReference("FAKE".into()),
            trigger: spin_config::TriggerConfig::Http(spin_config::HttpConfig {
                route: "/test".to_string(),
                executor: Some(spin_config::HttpExecutor::Spin),
            }),
            wasm: spin_config::WasmConfig {
                environment: HashMap::new(),
                mounts: vec![],
                allowed_http_hosts: vec![],
            },
        }
    }
}
