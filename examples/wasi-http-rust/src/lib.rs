wit_bindgen::generate!("proxy" in "../../wit/preview2");

use self::http::{Http, IncomingRequest, ResponseOutparam};
use streams2 as streams;
use types2 as types;

const READ_SIZE: u64 = 16 * 1024;

struct Component;

impl Http for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        let method = types::incoming_request_method(request);
        let path = types::incoming_request_path_with_query(request);
        println!("{method:?} {path:?}");
        let response =
            types::new_outgoing_response(200, types::new_fields(&[("foo", "bar".as_bytes())]));
        types::set_response_outparam(response_out, Ok(response))
            .expect("response outparam should be settable");
        let request_body =
            types::incoming_request_consume(request).expect("request should be consumable");
        let response_body =
            types::outgoing_response_write(response).expect("response should be writable");
        loop {
            let (buffer, end) =
                streams::blocking_read(request_body, READ_SIZE).expect("read should succeed");
            let mut offset = 0;
            loop {
                let count = usize::try_from(
                    streams::blocking_write(response_body, &buffer[offset..])
                        .expect("write should succeed"),
                )
                .unwrap();

                offset += count;
                if offset == buffer.len() {
                    break;
                }
            }
            if end {
                break;
            }
        }
        types::finish_incoming_stream(request_body);
        types::finish_outgoing_stream(response_body, None);
    }
}

export_proxy!(Component);
