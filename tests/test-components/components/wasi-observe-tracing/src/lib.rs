wit_bindgen::generate!({
    path: "../../../../wit",
    world: "wasi:observe/imports@0.2.0-draft",
    generate_all,
});

use spin_sdk::{
    http::{Method, Params, Request, Response, Router},
    http_component,
};
use wasi::{
    clocks0_2_0::wall_clock::now,
    observe::tracer::{self, KeyValue, Link, StartOptions, Value},
};

#[http_component]
fn handle(req: http::Request<()>) -> Response {
    let mut router = Router::new();
    router.get("/nested-spans", nested_spans);
    router.get("/drop-semantics", drop_semantics);
    router.get("/setting-attributes", setting_attributes);
    router.get_async("/host-guest-host", host_guest_host);
    router.get("/events", events);
    router.get_async("/child-outlives-parent", child_outlives_parent);
    router.get("/links", links);
    router.get_async("/root-span", root_span);
    router.handle(req)
}

fn nested_spans(_req: Request, _params: Params) -> Response {
    let span = tracer::start("outer_func", None);
    inner_func();
    span.end(None);
    Response::new(200, "")
}

fn inner_func() {
    let span = tracer::start("inner_func", None);
    span.end(None);
}

fn drop_semantics(_req: Request, _params: Params) -> Response {
    let _span = tracer::start("drop_semantics", None);
    Response::new(200, "")
    // _span will drop here and should be ended
}

fn setting_attributes(_req: Request, _params: Params) -> Response {
    let span = tracer::start("setting_attributes", None);
    span.set_attributes(&[KeyValue {
        key: "foo".to_string(),
        value: Value::String("bar".to_string()),
    }]);
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
    span.end(None);
    Response::new(200, "")
}

async fn host_guest_host(_req: Request, _params: Params) -> Response {
    let span = tracer::start("guest", None);

    make_request().await;
    span.end(None);

    Response::new(200, "")
}

fn events(_req: Request, _params: Params) -> Response {
    let span = tracer::start("events", None);
    span.add_event("basic-event", None, None);
    span.add_event(
        "event-with-attributes",
        None,
        Some(&[KeyValue {
            key: "foo".to_string(),
            value: Value::Bool(true),
        }]),
    );
    let mut now_plus = now();
    now_plus.seconds += 1;
    span.add_event("event-with-timestamp", Some(now_plus), None);
    span.end(None);
    Response::new(200, "")
}

async fn child_outlives_parent(_req: Request, _params: Params) -> Response {
    let span = tracer::start("parent", None);
    let span2 = tracer::start("child", None);
    span.end(None);
    // Make a host call to test span reparenting when we're messing with the active span stack
    make_request().await;

    span2.end(None);
    Response::new(200, "")
}

fn links(_req: Request, _params: Params) -> Response {
    let first = tracer::start("first", None);
    first.end(None);
    let second = tracer::start("second", None);
    second.add_link(&Link {
        span_context: first.span_context(),
        attributes: vec![KeyValue {
            key: "foo".to_string(),
            value: Value::String("bar".to_string()),
        }],
    });
    second.end(None);
    Response::new(200, "")
}

async fn root_span(_req: Request, _params: Params) -> Response {
    let span1 = tracer::start("parent", None);
    make_request().await;
    let root = tracer::start(
        "root",
        Some(&StartOptions {
            new_root: true,
            span_kind: None,
            attributes: None,
            links: None,
            timestamp: None,
        }),
    );
    make_request().await;
    root.end(None);
    span1.end(None);
    make_request().await;
    Response::new(200, "")
}

async fn make_request() {
    let req = Request::builder()
        .method(Method::Get)
        .uri("https://asdf.com")
        .build();
    let _res: Response = spin_sdk::http::send(req).await.unwrap();
}
