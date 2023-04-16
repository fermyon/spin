use super::{Request, Response, Result};
use routefinder::{Captures, Router as MethodRouter};
use std::{collections::HashMap, fmt::Display};

type Handler = dyn Fn(Request, Params) -> Result<Response>;

/// Route parameters extracted from a URI that match a route pattern.
pub type Params = Captures<'static, 'static>;

/// The Spin SDK HTTP router.
pub struct Router {
    methods_map: HashMap<http_types::Method, MethodRouter<Box<Handler>>>,
    any_methods: MethodRouter<Box<Handler>>,
}

impl Default for Router {
    fn default() -> Router {
        Router::new()
    }
}

impl Display for Router {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Registered routes:")?;
        for (method, router) in &self.methods_map {
            for route in router.iter() {
                writeln!(f, "- {}: {}", method, route.0)?;
            }
        }
        Ok(())
    }
}

struct RouteMatch<'a> {
    params: Captures<'static, 'static>,
    handler: &'a Handler,
}

impl Router {
    /// Dispatches a request to the appropriate handler along with the URI parameters.
    pub fn handle(&self, request: Request) -> Result<Response> {
        let method = request.method().to_owned();
        let path = request.uri().path().to_owned();
        let RouteMatch { params, handler } = self.find(&path, method);
        handler(request, params)
    }

    fn find(&self, path: &str, method: http_types::Method) -> RouteMatch<'_> {
        let best_match = self
            .methods_map
            .get(&method)
            .and_then(|r| r.best_match(path));

        if let Some(m) = best_match {
            let params = m.captures().into_owned();
            let handler = m.handler();
            return RouteMatch { handler, params };
        }

        let best_match = self.any_methods.best_match(path);

        match best_match {
            Some(m) => {
                let params = m.captures().into_owned();
                let handler = m.handler();
                RouteMatch { handler, params }
            }
            None if method == http_types::Method::HEAD => {
                // If it is a HTTP HEAD request then check if there is a callback in the methods map
                // if not then fallback to the behavior of HTTP GET else proceed as usual
                self.find(path, http_types::Method::GET)
            }
            None => {
                // Handle the failure case where no match could be resolved.
                self.fail(path, method)
            }
        }
    }

    // Helper function to handle the case where a best match couldn't be resolved.
    fn fail(&self, path: &str, method: http_types::Method) -> RouteMatch<'_> {
        // First, filter all routers to determine if the path can match but the provided method is not allowed.
        let is_method_not_allowed = self
            .methods_map
            .iter()
            .filter(|(k, _)| **k != method)
            .any(|(_, r)| r.best_match(path).is_some());

        if is_method_not_allowed {
            // If this `path` can be handled by a callback registered with a different HTTP method
            // should return 405 Method Not Allowed
            RouteMatch {
                handler: &method_not_allowed,
                params: Captures::default(),
            }
        } else {
            // ... Otherwise, nothing matched so 404.
            RouteMatch {
                handler: &not_found,
                params: Captures::default(),
            }
        }
    }

    /// Register a handler at the path for all methods.
    pub fn any<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request, Params) -> Result<Response> + 'static,
    {
        self.any_methods.add(path, Box::new(handler)).unwrap();
    }

    /// Register a handler at the path for the specified HTTP method.
    pub fn add<F>(&mut self, path: &str, method: http_types::Method, handler: F)
    where
        F: Fn(Request, Params) -> Result<Response> + 'static,
    {
        self.methods_map
            .entry(method)
            .or_insert_with(MethodRouter::new)
            .add(path, Box::new(handler))
            .unwrap();
    }

    /// Register a handler at the path for the HTTP GET method.
    pub fn get<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request, Params) -> Result<Response> + 'static,
    {
        self.add(path, http_types::Method::GET, handler)
    }

    /// Register a handler at the path for the HTTP HEAD method.
    pub fn head<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request, Params) -> Result<Response> + 'static,
    {
        self.add(path, http_types::Method::HEAD, handler)
    }

    /// Register a handler at the path for the HTTP POST method.
    pub fn post<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request, Params) -> Result<Response> + 'static,
    {
        self.add(path, http_types::Method::POST, handler)
    }

    /// Register a handler at the path for the HTTP DELETE method.
    pub fn delete<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request, Params) -> Result<Response> + 'static,
    {
        self.add(path, http_types::Method::DELETE, handler)
    }

    /// Register a handler at the path for the HTTP PUT method.
    pub fn put<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request, Params) -> Result<Response> + 'static,
    {
        self.add(path, http_types::Method::PUT, handler)
    }

    /// Register a handler at the path for the HTTP PATCH method.
    pub fn patch<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request, Params) -> Result<Response> + 'static,
    {
        self.add(path, http_types::Method::PATCH, handler)
    }

    /// Construct a new Router.
    pub fn new() -> Self {
        Router {
            methods_map: HashMap::default(),
            any_methods: MethodRouter::new(),
        }
    }
}

fn not_found(_req: Request, _params: Params) -> Result<Response> {
    Ok(http_types::Response::builder()
        .status(http_types::StatusCode::NOT_FOUND)
        .body(None)
        .unwrap())
}

fn method_not_allowed(_req: Request, _params: Params) -> Result<Response> {
    Ok(http_types::Response::builder()
        .status(http_types::StatusCode::METHOD_NOT_ALLOWED)
        .body(None)
        .unwrap())
}

/// A macro to help with constructing a Router from a stream of tokens.
#[macro_export]
macro_rules! http_router {
    ($($method:tt $path:literal => $h:expr),*) => {
        {
            let mut router = spin_sdk::http::Router::new();
            $(
                spin_sdk::http_router!(@build router $method $path => $h);
            )*
            router
        }
    };
    (@build $r:ident HEAD $path:literal => $h:expr) => {
        $r.head($path, $h);
    };
    (@build $r:ident GET $path:literal => $h:expr) => {
        $r.get($path, $h);
    };
    (@build $r:ident PUT $path:literal => $h:expr) => {
        $r.put($path, $h);
    };
    (@build $r:ident POST $path:literal => $h:expr) => {
        $r.post($path, $h);
    };
    (@build $r:ident PATCH $path:literal => $h:expr) => {
        $r.patch($path, $h);
    };
    (@build $r:ident DELETE $path:literal => $h:expr) => {
        $r.delete($path, $h);
    };
    (@build $r:ident _ $path:literal => $h:expr) => {
        $r.any($path, $h);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(method: http_types::Method, path: &str) -> Request {
        http_types::Request::builder()
            .method(method)
            .uri(path)
            .body(None)
            .unwrap()
    }

    fn echo_param(req: Request, params: Params) -> Result<Response> {
        match params.get("x") {
            Some(path) => Ok(http_types::Response::builder()
                .status(http_types::StatusCode::OK)
                .body(Some(path.to_string().into()))?),
            None => not_found(req, params),
        }
    }

    #[test]
    fn test_method_not_allowed() {
        let mut router = Router::default();
        router.get("/:x", echo_param);

        let req = make_request(http_types::Method::POST, "/foobar");
        let res = router.handle(req).unwrap();
        assert_eq!(res.status(), http_types::StatusCode::METHOD_NOT_ALLOWED);
    }

    #[test]
    fn test_not_found() {
        fn h1(_req: Request, _params: Params) -> Result<Response> {
            Ok(http_types::Response::builder().status(200).body(None)?)
        }

        let mut router = Router::default();
        router.get("/h1/:param", h1);

        let req = make_request(http_types::Method::GET, "/h1/");
        let res = router.handle(req).unwrap();
        assert_eq!(res.status(), http_types::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_multi_param() {
        fn multiply(_req: Request, params: Params) -> Result<Response> {
            let x: i64 = params.get("x").unwrap().parse()?;
            let y: i64 = params.get("y").unwrap().parse()?;
            Ok(http_types::Response::builder()
                .status(http_types::StatusCode::OK)
                .body(Some(format!("{result}", result = x * y).into()))?)
        }

        let mut router = Router::default();
        router.get("/multiply/:x/:y", multiply);

        let req = make_request(http_types::Method::GET, "/multiply/2/4");
        let res = router.handle(req).unwrap();

        assert_eq!(res.into_body().unwrap(), "8".to_string());
    }

    #[test]
    fn test_param() {
        let mut router = Router::default();
        router.get("/:x", echo_param);

        let req = make_request(http_types::Method::GET, "/y");
        let res = router.handle(req).unwrap();

        assert_eq!(res.into_body().unwrap(), "y".to_string());
    }

    #[test]
    fn test_wildcard() {
        fn echo_wildcard(req: Request, params: Params) -> Result<Response> {
            match params.wildcard() {
                Some(path) => Ok(http_types::Response::builder()
                    .status(http_types::StatusCode::OK)
                    .body(Some(path.to_string().into()))?),
                None => not_found(req, params),
            }
        }

        let mut router = Router::default();
        router.get("/*", echo_wildcard);

        let req = make_request(http_types::Method::GET, "/foo/bar");
        let res = router.handle(req).unwrap();
        assert_eq!(res.status(), http_types::StatusCode::OK);
        assert_eq!(res.into_body().unwrap(), "foo/bar".to_string());
    }

    #[test]
    fn test_wildcard_last_segment() {
        let mut router = Router::default();
        router.get("/:x/*", echo_param);
        
        let req = make_request(http_types::Method::GET, "/foo/bar");
        let res = router.handle(req).unwrap();
        assert_eq!(res.into_body().unwrap(), "foo".to_string());
    }

    #[test]
    fn test_router_display() {
        let mut router = Router::default();
        router.get("/:x", echo_param);
        
        let expected = "Registered routes:\n- GET: /:x\n";
        let actual = format!("{}", router);
        
        assert_eq!(actual.as_str(), expected);
    }
    
    #[test]
    fn test_ambiguous_wildcard_vs_star() {
        fn h1(_req: Request, _params: Params) -> Result<Response> {
            Ok(http_types::Response::builder()
                .status(http_types::StatusCode::OK)
                .body(Some("one/two".into()))?)
        }

        fn h2(_req: Request, _params: Params) -> Result<Response> {
            Ok(http_types::Response::builder()
                .status(http_types::StatusCode::OK)
                .body(Some("posts/*".into()))?)
        }

        let mut router = Router::default();
        router.get("/:one/:two", h1);
        router.get("/posts/*", h2);

        let req = make_request(http_types::Method::GET, "/posts/2");
        let res = router.handle(req).unwrap();

        assert_eq!(res.into_body().unwrap(), "posts/*".to_string());
    }
}
