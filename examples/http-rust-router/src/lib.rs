use anyhow::Result;
use spin_sdk::{
    http::{IntoResponse, Params, Request, Response, Router},
    http_component,
};

/// A Spin HTTP component that internally routes requests.
#[http_component]
fn handle_route(req: Request) -> Response {
    let mut router = Router::new();
    router.get("/goodbye/:planet", api::goodbye_planet);
    router.get_async("/hello/:planet", api::hello_planet);
    router.any_async("/*", api::echo_wildcard);
    router.handle(req)
}

mod api {
    use super::*;

    // /goodbye/:planet
    pub fn goodbye_planet(_req: Request, params: Params) -> Result<impl IntoResponse> {
        let planet = params.get("planet").expect("PLANET");
        Ok(Response::new(200, planet.to_string()))
    }

    // /hello/:planet
    pub async fn hello_planet(_req: Request, params: Params) -> Result<impl IntoResponse> {
        let planet = params.get("planet").expect("PLANET");
        Ok(Response::new(200, planet.to_string()))
    }

    // /*
    pub async fn echo_wildcard(_req: Request, params: Params) -> Result<impl IntoResponse> {
        let capture = params.wildcard().unwrap_or_default();
        Ok(Response::new(200, capture.to_string()))
    }
}
