wit_bindgen::generate!("proxy" in "../../wit/wasi-http");

use self::{
    exports::wasi::http::incoming_handler::IncomingHandler,
    wasi::{
        http::{
            outgoing_handler,
            types2::{self as types, IncomingRequest, Method, ResponseOutparam, Scheme},
        },
        io::streams2::{self as streams, StreamStatus},
        poll::poll2 as poll,
    },
};
use anyhow::Result;
use std::str;
use url::Url;

const READ_SIZE: u64 = 16 * 1024;

struct Component;

impl IncomingHandler for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        do_handle(request, response_out).unwrap()
    }
}

fn do_handle(request: IncomingRequest, response_out: ResponseOutparam) -> Result<()> {
    let method = types::incoming_request_method(request);
    let path = types::incoming_request_path_with_query(request);
    let headers = types::fields_entries(types::incoming_request_headers(request));
    println!(
        "{method:?} {path:?} {:?}",
        headers
            .iter()
            .map(|(k, v)| (k, String::from_utf8_lossy(v)))
            .collect::<Vec<_>>()
    );

    match (method, path.as_deref()) {
        (Method::Post, Some("/echo")) => {
            let poll = headers.iter().any(|(k, v)| k == "poll" && v == b"true");
            let response =
                types::new_outgoing_response(200, types::new_fields(&[("foo", "bar".as_bytes())]))?;

            types::set_response_outparam(response_out, Ok(response))
                .expect("response outparam should be settable");

            let request_body =
                types::incoming_request_consume(request).expect("request should be consumable");
            let response_body =
                types::outgoing_response_write(response).expect("response should be writable");

            let total = if poll {
                println!("using polling API");
                pipe_polling(request_body, response_body)
            } else {
                println!("using blocking API");
                pipe_blocking(request_body, response_body)
            };

            println!("echoed {total} bytes");
            types::finish_incoming_stream(request_body);
            types::finish_outgoing_stream(response_body);
        }

        (Method::Get, Some("/proxy")) => {
            let url = headers
                .iter()
                .find_map(|(k, v)| (k == "url").then_some(v))
                .and_then(|v| str::from_utf8(v).ok())
                .and_then(|v| Url::parse(v).ok());

            if let Some(url) = url {
                let outgoing_request = types::new_outgoing_request(
                    &Method::Get,
                    Some(url.path()),
                    Some(&match url.scheme() {
                        "http" => Scheme::Http,
                        "https" => Scheme::Https,
                        scheme => Scheme::Other(scheme.into()),
                    }),
                    url.host().map(|host| host.to_string()).as_deref(),
                    types::new_fields(&[]),
                )?;

                let incoming_response = outgoing_handler::handle(outgoing_request, None);

                types::outgoing_request_write(outgoing_request)
                    .expect("request should be writable");

                let incoming_response_pollable =
                    types::listen_to_future_incoming_response(incoming_response);

                let incoming_response = loop {
                    if let Some(incoming_response) =
                        types::future_incoming_response_get(incoming_response)
                    {
                        break incoming_response;
                    } else {
                        poll::poll_oneoff(&[incoming_response_pollable]);
                    }
                };

                match incoming_response {
                    Ok(incoming_response) => {
                        let response = types::new_outgoing_response(
                            types::incoming_response_status(incoming_response),
                            types::incoming_response_headers(incoming_response),
                        )?;

                        types::set_response_outparam(response_out, Ok(response))
                            .expect("response outparam should be settable");

                        let incoming_response_body =
                            types::incoming_response_consume(incoming_response)
                                .expect("response should be consumable");
                        let response_body = types::outgoing_response_write(response)
                            .expect("response should be writable");

                        pipe_blocking(incoming_response_body, response_body);

                        types::finish_incoming_stream(incoming_response_body);
                        types::finish_outgoing_stream(response_body);
                    }

                    Err(_error) => {
                        todo!()
                    }
                }
            } else {
                todo!()
            }
        }

        _ => {
            let response = types::new_outgoing_response(405, types::new_fields(&[]))?;

            types::set_response_outparam(response_out, Ok(response))
                .expect("response outparam should be settable");

            types::finish_outgoing_stream(
                types::outgoing_response_write(response).expect("response should be writable"),
            );
        }
    }

    Ok(())
}

export_proxy!(Component);

fn pipe_blocking(input: streams::InputStream, output: streams::OutputStream) -> usize {
    let mut total = 0;
    loop {
        let (buffer, status) =
            streams::blocking_read(input, READ_SIZE).expect("read should succeed");
        if buffer.is_empty() && status == StreamStatus::Ended {
            break total;
        }

        total += buffer.len();
        let mut offset = 0;
        loop {
            assert!(!buffer[offset..].is_empty());

            let count = usize::try_from(
                streams::blocking_write(output, &buffer[offset..]).expect("write should succeed"),
            )
            .unwrap();

            assert!(count > 0);

            offset += count;
            if offset == buffer.len() {
                break;
            }
        }

        if let StreamStatus::Ended = status {
            break total;
        }
    }
}

fn pipe_polling(input: streams::InputStream, output: streams::OutputStream) -> usize {
    let input_pollable = streams::subscribe_to_input_stream(input);
    let output_pollable = streams::subscribe_to_output_stream(output);

    let mut total = 0;
    'outer: loop {
        let (buffer, status) = loop {
            let (buffer, status) = streams::read(input, READ_SIZE).expect("read should succeed");

            if buffer.is_empty() {
                if let StreamStatus::Ended = status {
                    break 'outer total;
                }

                poll::poll_oneoff(&[input_pollable]);
            } else {
                break (buffer, status);
            }
        };

        total += buffer.len();
        let mut offset = 0;
        loop {
            assert!(!buffer[offset..].is_empty());

            let count = usize::try_from(
                streams::write(output, &buffer[offset..]).expect("write should succeed"),
            )
            .unwrap();

            if count == 0 {
                poll::poll_oneoff(&[output_pollable]);
            } else {
                offset += count;
                if offset == buffer.len() {
                    break;
                }
            }
        }

        if let StreamStatus::Ended = status {
            break total;
        }
    }
}
