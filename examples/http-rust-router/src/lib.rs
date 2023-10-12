use anyhow::Result;
use spin_sdk::{
    http::{Params, Request, Response, Router},
    http_component,
};

/// A Spin HTTP component that internally routes requests.
#[http_component]
fn handle_route(req: Request) -> Response {
    let mut router = Router::new();
    router.get("/hello/:planet", api::hello_planet);
    router.any("/*", api::echo_wildcard);
    router.handle(req)
}

mod api {
    use super::*;

    // /hello/:planet
    pub fn hello_planet(_req: Request, params: Params) -> Result<http::Response<String>> {
        let planet = params.get("planet").expect("PLANET");

        Ok(http::Response::builder()
            .status(http::StatusCode::OK)
            .body(planet.to_string())?)
    }

    // /*
    pub fn echo_wildcard(_req: Request, params: Params) -> Result<http::Response<String>> {
        let capture = params.wildcard().unwrap_or_default();
        Ok(http::Response::builder()
            .status(http::StatusCode::OK)
            .body(capture.to_string())?)
    }
}
