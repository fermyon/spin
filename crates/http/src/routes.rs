//! Route matching for the HTTP trigger.

#![deny(missing_docs)]

use anyhow::{bail, Result};
use indexmap::IndexMap;
use spin_config::{Configuration, CoreComponent};
use std::fmt::Debug;
use tracing::{instrument, log};

// TODO
// The current implementation of the router clones the components, which could
// become costly if we have a lot of components.
// The router should borrow the components, which needs to introduce a lifetime
// paramter which surfaces in the HTTP trigger (and which needs a &'static because
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
    /// Build a router based on application configuration.
    #[instrument]
    pub(crate) fn build(app: &Configuration<CoreComponent>) -> Result<Self> {
        let mut routes = IndexMap::new();
        for component in &app.components {
            let spin_config::TriggerConfig::Http(trigger) = &component.trigger;
            log::info!("Trying route path {}", trigger.route);

            routes.insert(RoutePattern::from(trigger.route.clone()), component.clone());
        }

        log::info!(
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

    /// Return the component that should handle the given path, or an error
    /// if no component matches.
    /// If there are multiple possible components registered for the same route or
    /// wildcard, return the last one in the components vector.
    #[instrument]
    pub(crate) fn route<S: Into<String> + Debug>(&self, p: S) -> Result<CoreComponent> {
        let p = p.into();

        // TODO
        // A filter would be very nice here to avoid all the clones.
        let mut res = vec![];
        for (rp, c) in &self.routes {
            if rp.matches(p.clone()) {
                res.push(c.clone());
            }
        }

        match res.last() {
            Some(c) => Ok(c.clone()),
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
    /// Return a RoutePattern given a path fragment.
    pub fn from<S: Into<String>>(path: S) -> Self {
        let path = path.into();
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

    /// Strip the trailing slash from a string.
    fn sanitize<S: Into<String>>(s: S) -> String {
        let s = s.into();
        // TODO
        // This only strips a single trailing slash.
        // Should we attempt to strip all trailing slashes?
        match s.strip_suffix("/") {
            Some(s) => s.into(),
            None => s,
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

        let rp = RoutePattern::from("/foo/bar");
        assert_eq!(rp.matches("/foo/bar"), true);
        assert_eq!(rp.matches("/foo/bar/"), true);
        assert_eq!(rp.matches("/foo"), false);
        assert_eq!(rp.matches("/foo/bar/thisshouldbefalse"), false);
        assert_eq!(rp.matches("/abc"), false);
    }

    #[test]
    fn test_pattern_route() {
        let rp = RoutePattern::from("/...");
        assert_eq!(rp.matches("/foo/bar/"), true);
        assert_eq!(rp.matches("/foo"), true);
        assert_eq!(rp.matches("/foo/bar/baz"), true);
        assert_eq!(rp.matches("/this/should/really/match/everything/"), true);
        assert_eq!(rp.matches("/"), true);

        let rp = RoutePattern::from("/foo/...");
        assert_eq!(rp.matches("/foo/bar/"), true);
        assert_eq!(rp.matches("/foo"), true);
        assert_eq!(rp.matches("/foo/bar/baz"), true);
        assert_eq!(
            rp.matches("/this/should/really/not/match/everything/"),
            false
        );
        assert_eq!(rp.matches("/"), false);
    }

    #[test]
    fn test_router() -> Result<()> {
        let mut routes = IndexMap::new();

        let foo_component = named_component("foo");
        let foobar_component = named_component("foobar");

        routes.insert(RoutePattern::from("/foo"), foo_component);
        routes.insert(RoutePattern::from("/foo/bar"), foobar_component);

        let r = Router { routes };

        assert_eq!(r.route("/foo")?.id, "foo".to_string());
        assert_eq!(r.route("/foo/bar")?.id, "foobar".to_string());

        let mut routes = IndexMap::new();

        let all_component = named_component("all");
        routes.insert(RoutePattern::from("/..."), all_component);

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

        routes.insert(RoutePattern::from("/one/..."), one_wildcard);
        routes.insert(RoutePattern::from("/one/two/..."), onetwo_wildcard);
        routes.insert(
            RoutePattern::from("/one/two/three/..."),
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
            RoutePattern::from("/one/two/three/..."),
            onetwothree_wildcard,
        );
        routes.insert(RoutePattern::from("/one/two/..."), onetwo_wildcard);
        routes.insert(RoutePattern::from("/one/..."), one_wildcard);

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
            ..Default::default()
        }
    }
}
