use anyhow::Result;
use spin_sdk::{
    http::{IntoResponse, Params, Router},
    http_component,
};

/// A Spin HTTP component that internally routes requests.
#[http_component]
fn handle_route(req: http::Request<()>) -> impl IntoResponse {
    let mut router = Router::new();
    router.get("/hello/:planet", api::hello_planet);
    router.any("/*", api::echo_wildcard);
    router.handle(req)
}

mod api {
    use super::*;

    // /hello/:planet
    pub fn hello_planet(_req: http::Request<()>, params: Params) -> Result<impl IntoResponse> {
        let planet = params.get("planet").expect("PLANET");

        Ok(http::Response::builder()
            .status(200)
            .body(planet.to_string())?)
    }

    // /*
    pub fn echo_wildcard(_req: http::Request<()>, params: Params) -> Result<impl IntoResponse> {
        let capture = params.wildcard().unwrap_or_default();
        Ok(http::Response::builder()
            .status(200)
            .body(capture.to_string())?)
    }
}
