wit_bindgen::generate!({
    path: "wit",
    world: "wasi:http/proxy@0.2.0-rc-2023-12-05",
    exports: {
        "wasi:http/incoming-handler": Component
    }
});

use {
    exports::wasi::http::incoming_handler,
    url::Url,
    wasi::{
        http::{
            outgoing_handler,
            types::{
                Headers, IncomingRequest, Method, OutgoingBody, OutgoingRequest, OutgoingResponse,
                ResponseOutparam, Scheme,
            },
        },
        io::streams::StreamError,
    },
};

struct Component;

impl incoming_handler::Guest for Component {
    fn handle(request: IncomingRequest, outparam: ResponseOutparam) {
        let headers = request.headers().entries();

        if let Some(url) = headers.iter().find_map(|(k, v)| {
            (k == "url")
                .then_some(v)
                .and_then(|v| std::str::from_utf8(v).ok())
                .and_then(|v| Url::parse(v).ok())
        }) {
            let outgoing_request = OutgoingRequest::new(Headers::new());
            outgoing_request.set_method(&Method::Get).unwrap();
            outgoing_request
                .set_path_with_query(Some(url.path()))
                .unwrap();
            outgoing_request
                .set_scheme(Some(&match url.scheme() {
                    "http" => Scheme::Http,
                    "https" => Scheme::Https,
                    scheme => Scheme::Other(scheme.into()),
                }))
                .unwrap();
            outgoing_request
                .set_authority(Some(url.authority()))
                .unwrap();

            let response = outgoing_handler::handle(outgoing_request, None).unwrap();
            let response = loop {
                if let Some(response) = response.get() {
                    break response.unwrap().unwrap();
                } else {
                    response.subscribe().block()
                }
            };

            let incoming_body = response.consume().unwrap();
            let incoming_stream = incoming_body.stream().unwrap();
            let status = response.status();
            let response = OutgoingResponse::new(response.headers().clone());
            response.set_status_code(status).unwrap();
            let outgoing_body = response.body().unwrap();
            {
                let outgoing_stream = outgoing_body.write().unwrap();
                ResponseOutparam::set(outparam, Ok(response));

                loop {
                    match incoming_stream.read(1024) {
                        Ok(buffer) => {
                            if buffer.is_empty() {
                                incoming_stream.subscribe().block();
                            } else {
                                outgoing_stream.blocking_write_and_flush(&buffer).unwrap();
                            }
                        }
                        Err(StreamError::Closed) => break,
                        Err(StreamError::LastOperationFailed(error)) => {
                            panic!("{}", error.to_debug_string())
                        }
                    }
                }
            }

            OutgoingBody::finish(outgoing_body, None).unwrap();
        } else {
            let response = OutgoingResponse::new(Headers::new());
            response.set_status_code(400).unwrap();
            let body = response.body().unwrap();

            ResponseOutparam::set(outparam, Ok(response));

            OutgoingBody::finish(body, None).unwrap();
        }
    }
}
