wit_bindgen::generate!("proxy" in "../../wit/preview2");

use self::http::{Http, IncomingRequest, ResponseOutparam};
use poll2 as poll;
use streams2 as streams;
use types2 as types;

const READ_SIZE: u64 = 16 * 1024;

struct Component;

impl Http for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
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

        let poll = headers.iter().any(|(k, v)| k == "poll" && v == b"true");
        let response =
            types::new_outgoing_response(200, types::new_fields(&[("foo", "bar".as_bytes())]));

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
        types::finish_outgoing_stream(response_body, None);
    }
}

export_proxy!(Component);

fn pipe_blocking(input: streams::InputStream, output: streams::OutputStream) -> usize {
    let mut total = 0;
    loop {
        let (buffer, end) = streams::blocking_read(input, READ_SIZE).expect("read should succeed");
        if buffer.is_empty() && end {
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

        if end {
            break total;
        }
    }
}

fn pipe_polling(input: streams::InputStream, output: streams::OutputStream) -> usize {
    let input_pollable = streams::subscribe_to_input_stream(input);
    let output_pollable = streams::subscribe_to_output_stream(output);

    let mut total = 0;
    'outer: loop {
        let (buffer, end) = loop {
            let (buffer, end) = streams::read(input, READ_SIZE).expect("read should succeed");

            if buffer.is_empty() {
                if end {
                    break 'outer total;
                }

                poll::poll_oneoff(&[input_pollable]);
            } else {
                break (buffer, end);
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

        if end {
            break total;
        }
    }
}
