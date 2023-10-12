#![allow(dead_code, unused_imports)]
use spin_sdk::{
    http::{IntoResponse, Params, Request},
    http_component, http_router,
};

#[http_component]
fn handle_route(req: http::Request<()>) -> impl IntoResponse {
    let router = http_router! {
        GET "/hello/:planet" => api::hello_planet,
        _   "/*"             => |_req: Request, params| {
            let capture = params.wildcard().unwrap_or_default();
            http::Response::builder()
                .status(http::StatusCode::OK)
                .body(Some(capture.to_string()))
                .unwrap()
        }
    };
    router.handle(req)
}

mod api {
    use super::*;

    // /hello/:planet
    pub fn hello_planet(
        _req: http::Request<()>,
        params: Params,
    ) -> anyhow::Result<impl IntoResponse> {
        let planet = params.get("planet").expect("PLANET");

        Ok(http::Response::builder()
            .status(http::StatusCode::OK)
            .body(Some(planet.to_string()))
            .unwrap())
    }
}
