use super::conversions::{IntoRequest, IntoResponse, TryFromRequest};
use super::{responses, Request, Response};
use routefinder::{Captures, Router as MethodRouter};
use std::{collections::HashMap, fmt::Display};

type Handler = dyn Fn(Request, Params) -> Response;

/// Route parameters extracted from a URI that match a route pattern.
pub type Params = Captures<'static, 'static>;

/// The Spin SDK HTTP router.
pub struct Router {
    methods_map: HashMap<hyperium::Method, MethodRouter<Box<Handler>>>,
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
    pub fn handle<R: IntoRequest>(&self, request: R) -> Response {
        let request = request.into_request();
        let method = request.method();
        let path = &request.uri().path();
        let RouteMatch { params, handler } = self.find(path, method.clone());
        handler(request, params)
    }

    fn find(&self, path: &str, method: hyperium::Method) -> RouteMatch<'_> {
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
            None if method == hyperium::Method::HEAD => {
                // If it is a HTTP HEAD request then check if there is a callback in the methods map
                // if not then fallback to the behavior of HTTP GET else proceed as usual
                self.find(path, hyperium::Method::GET)
            }
            None => {
                // Handle the failure case where no match could be resolved.
                self.fail(path, method)
            }
        }
    }

    // Helper function to handle the case where a best match couldn't be resolved.
    fn fail(&self, path: &str, method: hyperium::Method) -> RouteMatch<'_> {
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
    pub fn any<F, I, O>(&mut self, path: &str, handler: F)
    where
        F: Fn(I, Params) -> O + 'static,
        I: TryFromRequest,
        I::Error: IntoResponse,
        O: IntoResponse,
    {
        self.any_methods
            .add(
                path,
                Box::new(
                    move |req, params| match TryFromRequest::try_from_request(req) {
                        Ok(r) => handler(r, params).into_response(),
                        Err(e) => e.into_response(),
                    },
                ),
            )
            .unwrap();
    }

    /// Register a handler at the path for the specified HTTP method.
    pub fn add<F, I, O>(&mut self, path: &str, method: hyperium::Method, handler: F)
    where
        F: Fn(I, Params) -> O + 'static,
        I: TryFromRequest,
        I::Error: IntoResponse,
        O: IntoResponse,
    {
        self.methods_map
            .entry(method)
            .or_default()
            .add(
                path,
                Box::new(
                    move |req, params| match TryFromRequest::try_from_request(req) {
                        Ok(r) => handler(r, params).into_response(),
                        Err(e) => e.into_response(),
                    },
                ),
            )
            .unwrap();
    }

    /// Register a handler at the path for the HTTP GET method.
    pub fn get<F, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Resp + 'static,
        Req: TryFromRequest,
        Req::Error: IntoResponse,
        Resp: IntoResponse,
    {
        self.add(path, hyperium::Method::GET, handler)
    }

    /// Register a handler at the path for the HTTP HEAD method.
    pub fn head<F, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Resp + 'static,
        Req: TryFromRequest,
        Req::Error: IntoResponse,
        Resp: IntoResponse,
    {
        self.add(path, hyperium::Method::HEAD, handler)
    }

    /// Register a handler at the path for the HTTP POST method.
    pub fn post<F, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Resp + 'static,
        Req: TryFromRequest,
        Req::Error: IntoResponse,
        Resp: IntoResponse,
    {
        self.add(path, hyperium::Method::POST, handler)
    }

    /// Register a handler at the path for the HTTP DELETE method.
    pub fn delete<F, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Resp + 'static,
        Req: TryFromRequest,
        Req::Error: IntoResponse,
        Resp: IntoResponse,
    {
        self.add(path, hyperium::Method::DELETE, handler)
    }

    /// Register a handler at the path for the HTTP PUT method.
    pub fn put<F, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Resp + 'static,
        Req: TryFromRequest,
        Req::Error: IntoResponse,
        Resp: IntoResponse,
    {
        self.add(path, hyperium::Method::PUT, handler)
    }

    /// Register a handler at the path for the HTTP PATCH method.
    pub fn patch<F, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Resp + 'static,
        Req: TryFromRequest,
        Req::Error: IntoResponse,
        Resp: IntoResponse,
    {
        self.add(path, hyperium::Method::PATCH, handler)
    }

    /// Register a handler at the path for the HTTP OPTIONS method.
    pub fn options<F, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Resp + 'static,
        Req: TryFromRequest,
        Req::Error: IntoResponse,
        Resp: IntoResponse,
    {
        self.add(path, hyperium::Method::OPTIONS, handler)
    }

    /// Construct a new Router.
    pub fn new() -> Self {
        Router {
            methods_map: HashMap::default(),
            any_methods: MethodRouter::new(),
        }
    }
}

fn not_found(_req: Request, _params: Params) -> Response {
    responses::not_found().into_response()
}

fn method_not_allowed(_req: Request, _params: Params) -> Response {
    responses::method_not_allowed().into_response()
}

/// A macro to help with constructing a Router from a stream of tokens.
#[macro_export]
macro_rules! http_router {
    ($($method:tt $path:literal => $h:expr),*) => {
        {
            let mut router = $crate::http::Router::new();
            $(
                $crate::http_router!(@build router $method $path => $h);
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
    (@build $r:ident OPTIONS $path:literal => $h:expr) => {
        $r.options($path, $h);
    };
    (@build $r:ident _ $path:literal => $h:expr) => {
        $r.any($path, $h);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(method: hyperium::Method, path: &str) -> Request {
        hyperium::Request::builder()
            .method(method)
            .uri(path)
            .body(Vec::new())
            .unwrap()
    }

    fn echo_param(req: Request, params: Params) -> Response {
        match params.get("x") {
            Some(path) => hyperium::Response::builder()
                .status(200)
                .body(path.as_bytes().to_owned())
                .unwrap(),
            None => not_found(req, params),
        }
    }

    #[test]
    fn test_method_not_allowed() {
        let mut router = Router::default();
        router.get("/:x", echo_param);

        let req = make_request(hyperium::Method::POST, "/foobar");
        let res = router.handle(req);
        assert_eq!(res.status(), hyperium::StatusCode::METHOD_NOT_ALLOWED);
    }

    #[test]
    fn test_not_found() {
        fn h1(_req: Request, _params: Params) -> anyhow::Result<Response> {
            Ok(hyperium::Response::builder()
                .status(200)
                .body(Vec::new())
                .unwrap())
        }

        let mut router = Router::default();
        router.get("/h1/:param", h1);

        let req = make_request(hyperium::Method::GET, "/h1/");
        let res = router.handle(req);
        assert_eq!(res.status(), hyperium::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_multi_param() {
        fn multiply(_req: Request, params: Params) -> anyhow::Result<Response> {
            let x: i64 = params.get("x").unwrap().parse()?;
            let y: i64 = params.get("y").unwrap().parse()?;
            Ok(hyperium::Response::builder()
                .status(200)
                .body(format!("{result}", result = x * y).into_bytes())
                .unwrap())
        }

        let mut router = Router::default();
        router.get("/multiply/:x/:y", multiply);

        let req = make_request(hyperium::Method::GET, "/multiply/2/4");
        let res = router.handle(req);

        assert_eq!(res.into_body(), "8".to_owned().into_bytes());
    }

    #[test]
    fn test_param() {
        let mut router = Router::default();
        router.get("/:x", echo_param);

        let req = make_request(hyperium::Method::GET, "/y");
        let res = router.handle(req);

        assert_eq!(res.into_body(), "y".to_owned().into_bytes());
    }

    #[test]
    fn test_wildcard() {
        fn echo_wildcard(req: Request, params: Params) -> Response {
            match params.wildcard() {
                Some(path) => hyperium::Response::builder()
                    .status(200)
                    .body(path.as_bytes().to_owned())
                    .unwrap(),
                None => not_found(req, params),
            }
        }

        let mut router = Router::default();
        router.get("/*", echo_wildcard);

        let req = make_request(hyperium::Method::GET, "/foo/bar");
        let res = router.handle(req);
        assert_eq!(res.status(), hyperium::StatusCode::OK);
        assert_eq!(res.into_body(), "foo/bar".to_owned().into_bytes());
    }

    #[test]
    fn test_wildcard_last_segment() {
        let mut router = Router::default();
        router.get("/:x/*", echo_param);

        let req = make_request(hyperium::Method::GET, "/foo/bar");
        let res = router.handle(req);
        assert_eq!(res.into_body(), "foo".to_owned().into_bytes());
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
        fn h1(_req: Request, _params: Params) -> anyhow::Result<Response> {
            Ok(hyperium::Response::builder()
                .status(200)
                .body("one/two".into())
                .unwrap())
        }

        fn h2(_req: Request, _params: Params) -> anyhow::Result<Response> {
            Ok(hyperium::Response::builder()
                .status(200)
                .body("posts/*".into())
                .unwrap())
        }

        let mut router = Router::default();
        router.get("/:one/:two", h1);
        router.get("/posts/*", h2);

        let req = make_request(hyperium::Method::GET, "/posts/2");
        let res = router.handle(req);

        assert_eq!(res.into_body(), "posts/*".to_owned().into_bytes());
    }
}
