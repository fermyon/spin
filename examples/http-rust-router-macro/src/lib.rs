use spin_sdk::{
    http::{Params, Request, Response},
    http_component, http_router,
};

#[http_component]
fn handle_route(req: Request) -> anyhow::Result<Response> {
    let router = http_router! {
        GET "/hello/:planet" => api::hello_planet,
        _   "/*"             => |_req, params| {
            let capture = params.wildcard().unwrap_or_default();
            Ok(http::Response::builder()
                .status(http::StatusCode::OK)
                .body(Some(capture.to_string().into()))
                .unwrap())
        }
    };
    router.handle(req)
}

mod api {
    use super::*;

    // /hello/:planet
    pub fn hello_planet(_req: Request, params: Params) -> anyhow::Result<Response> {
        let planet = params.get("planet").expect("PLANET");

        Ok(http::Response::builder()
            .status(http::StatusCode::OK)
            .body(Some(planet.to_string().into()))
            .unwrap())
    }
}
