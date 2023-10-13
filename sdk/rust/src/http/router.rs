use super::{responses, IntoResponse, Method, Request, Response};
use routefinder::{Captures, Router as MethodRouter};
use std::{collections::HashMap, fmt::Display};

type Handler = dyn Fn(Request, Params) -> Response;

/// Route parameters extracted from a URI that match a route pattern.
pub type Params = Captures<'static, 'static>;

/// The Spin SDK HTTP router.
pub struct Router {
    methods_map: HashMap<Method, MethodRouter<Box<Handler>>>,
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
    pub fn handle<R: Into<Request>>(&self, request: R) -> Response {
        let request = request.into();
        let method = request.method;
        // TODO: get just the path
        let path = &request.uri;
        let RouteMatch { params, handler } = self.find(path, method);
        handler(request, params)
    }

    fn find(&self, path: &str, method: Method) -> RouteMatch<'_> {
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
            None if method == Method::Head => {
                // If it is a HTTP HEAD request then check if there is a callback in the methods map
                // if not then fallback to the behavior of HTTP GET else proceed as usual
                self.find(path, Method::Get)
            }
            None => {
                // Handle the failure case where no match could be resolved.
                self.fail(path, method)
            }
        }
    }

    // Helper function to handle the case where a best match couldn't be resolved.
    fn fail(&self, path: &str, method: Method) -> RouteMatch<'_> {
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
        I: From<Request>,
        O: IntoResponse,
    {
        self.any_methods
            .add(
                path,
                Box::new(move |req, params| handler(req.into(), params).into_response()),
            )
            .unwrap();
    }

    /// Register a handler at the path for the specified HTTP method.
    pub fn add<F, I, O>(&mut self, path: &str, method: Method, handler: F)
    where
        F: Fn(I, Params) -> O + 'static,
        I: TryFrom<Request>,
        O: IntoResponse,
    {
        self.methods_map
            .entry(method)
            .or_default()
            .add(
                path,
                Box::new(move |req, params| {
                    handler(req.try_into().unwrap_or_else(|_| panic!("TODO")), params)
                        .into_response()
                }),
            )
            .unwrap();
    }

    /// Register a handler at the path for the HTTP GET method.
    pub fn get<F, Req, Resp>(&mut self, path: &str, handler: F)
    where
        F: Fn(Req, Params) -> Resp + 'static,
        Req: TryFrom<Request>,
        Resp: IntoResponse,
    {
        self.add(path, Method::Get, handler)
    }

    /// Register a handler at the path for the HTTP HEAD method.
    pub fn head<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request, Params) -> anyhow::Result<Response> + 'static,
    {
        self.add(path, Method::Head, handler)
    }

    /// Register a handler at the path for the HTTP POST method.
    pub fn post<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request, Params) -> anyhow::Result<Response> + 'static,
    {
        self.add(path, Method::Post, handler)
    }

    /// Register a handler at the path for the HTTP DELETE method.
    pub fn delete<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request, Params) -> anyhow::Result<Response> + 'static,
    {
        self.add(path, Method::Delete, handler)
    }

    /// Register a handler at the path for the HTTP PUT method.
    pub fn put<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request, Params) -> anyhow::Result<Response> + 'static,
    {
        self.add(path, Method::Put, handler)
    }

    /// Register a handler at the path for the HTTP PATCH method.
    pub fn patch<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request, Params) -> anyhow::Result<Response> + 'static,
    {
        self.add(path, Method::Patch, handler)
    }

    /// Register a handler at the path for the HTTP OPTIONS method.
    pub fn options<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request, Params) -> anyhow::Result<Response> + 'static,
    {
        self.add(path, Method::Options, handler)
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
    responses::not_found()
}

fn method_not_allowed(_req: Request, _params: Params) -> Response {
    responses::method_not_allowed()
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

    fn make_request(method: Method, path: &str) -> Request {
        Request {
            method,
            uri: path.into(),
            headers: Vec::new(),
            params: Vec::new(),
            body: None,
        }
    }

    fn echo_param(req: Request, params: Params) -> Response {
        match params.get("x") {
            Some(path) => Response::new(200, Some(path.into())),
            None => not_found(req, params),
        }
    }

    #[test]
    fn test_method_not_allowed() {
        let mut router = Router::default();
        router.get("/:x", echo_param);

        let req = make_request(Method::Post, "/foobar");
        let res = router.handle(req);
        assert_eq!(res.status, http_types::StatusCode::METHOD_NOT_ALLOWED);
    }

    #[test]
    fn test_not_found() {
        fn h1(_req: Request, _params: Params) -> anyhow::Result<Response> {
            Ok(Response::new(200, None))
        }

        let mut router = Router::default();
        router.get("/h1/:param", h1);

        let req = make_request(Method::Get, "/h1/");
        let res = router.handle(req);
        assert_eq!(res.status, http_types::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_multi_param() {
        fn multiply(_req: Request, params: Params) -> anyhow::Result<Response> {
            let x: i64 = params.get("x").unwrap().parse()?;
            let y: i64 = params.get("y").unwrap().parse()?;
            Ok(Response::new(
                200,
                Some(format!("{result}", result = x * y).into()),
            ))
        }

        let mut router = Router::default();
        router.get("/multiply/:x/:y", multiply);

        let req = make_request(Method::Get, "/multiply/2/4");
        let res = router.handle(req);

        assert_eq!(res.body.unwrap(), "8".to_owned().into_bytes());
    }

    #[test]
    fn test_param() {
        let mut router = Router::default();
        router.get("/:x", echo_param);

        let req = make_request(Method::Get, "/y");
        let res = router.handle(req);

        assert_eq!(res.body.unwrap(), "y".to_owned().into_bytes());
    }

    #[test]
    fn test_wildcard() {
        fn echo_wildcard(req: Request, params: Params) -> Response {
            match params.wildcard() {
                Some(path) => Response::new(200, Some(path.to_string().into())),
                None => not_found(req, params),
            }
        }

        let mut router = Router::default();
        router.get("/*", echo_wildcard);

        let req = make_request(Method::Get, "/foo/bar");
        let res = router.handle(req);
        assert_eq!(res.status, http_types::StatusCode::OK);
        assert_eq!(res.body.unwrap(), "foo/bar".to_owned().into_bytes());
    }

    #[test]
    fn test_wildcard_last_segment() {
        let mut router = Router::default();
        router.get("/:x/*", echo_param);

        let req = make_request(Method::Get, "/foo/bar");
        let res = router.handle(req);
        assert_eq!(res.body.unwrap(), "foo".to_owned().into_bytes());
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
            Ok(Response::new(200, Some("one/two".into())))
        }

        fn h2(_req: Request, _params: Params) -> anyhow::Result<Response> {
            Ok(Response::new(200, Some("posts/*".into())))
        }

        let mut router = Router::default();
        router.get("/:one/:two", h1);
        router.get("/posts/*", h2);

        let req = make_request(Method::Get, "/posts/2");
        let res = router.handle(req);

        assert_eq!(res.body.unwrap(), "posts/*".to_owned().into_bytes());
    }
}
