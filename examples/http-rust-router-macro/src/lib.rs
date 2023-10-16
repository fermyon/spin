#![allow(dead_code, unused_imports)]
use spin_sdk::{
    http::{IntoResponse, Params, Request, Response},
    http_component, http_router,
};

#[http_component]
fn handle_route(req: Request) -> impl IntoResponse {
    let router = http_router! {
        GET "/hello/:planet" => api::hello_planet,
        _   "/*"             => |_req: Request, params| {
            let capture = params.wildcard().unwrap_or_default();
            Response::new(200, capture.to_string())
        }
    };
    router.handle(req)
}

mod api {
    use super::*;

    // /hello/:planet
    pub fn hello_planet(_req: Request, params: Params) -> anyhow::Result<impl IntoResponse> {
        let planet = params.get("planet").expect("PLANET");

        Ok(Response::new(200, planet.to_string()))
    }
}
