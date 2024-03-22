wit_bindgen::generate!({
    path: "../../../../wit",
    world: "wasi:observe/imports@0.2.0-draft",
    generate_all,
});

use spin_sdk::{
    http::{Params, Request, Response, Router},
    http_component,
};
use wasi::observe::traces::{KeyValue, Span, Value};

#[http_component]
fn handle(req: http::Request<()>) -> Response {
    let mut router = Router::new();
    router.get("/nested-spans", nested_spans);
    router.get("/drop-semantics", drop_semantics);
    router.get("/setting-attributes", setting_attributes);
    router.handle(req)
}

fn nested_spans(_req: Request, _params: Params) -> Response {
    let span = Span::start("outer_func");
    inner_func();
    span.end();
    Response::new(200, "")
}

fn inner_func() {
    let span = Span::start("inner_func");
    span.end();
}

fn drop_semantics(_req: Request, _params: Params) -> Response {
    let _span = Span::start("drop_semantics");
    Response::new(200, "")
    // _span will drop here and should be ended
}

fn setting_attributes(_req: Request, _params: Params) -> Response {
    let span = Span::start("setting_attributes");
    span.set_attribute(&KeyValue {
        key: "foo".to_string(),
        value: Value::String("bar".to_string()),
    });
    span.set_attributes(&[
        KeyValue {
            key: "foo".to_string(),
            value: Value::String("baz".to_string()),
        },
        KeyValue {
            key: "qux".to_string(),
            value: Value::StringArray(vec!["qaz".to_string(), "thud".to_string()]),
        },
    ]);
    span.end();
    Response::new(200, "")
}
