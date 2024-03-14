use spin_sdk::http::{Fields, IncomingRequest, OutgoingResponse, ResponseOutparam};
use spin_sdk::http_component;

use futures::{SinkExt, StreamExt};

/// A simple Spin HTTP component.
#[http_component]
async fn handle_back_end(req: IncomingRequest, resp: ResponseOutparam) {
    let mut reqbod = req.into_body_stream();

    let og = OutgoingResponse::new(200, &Fields::new(&[]));
    let mut ogbod = og.take_body();
    resp.set(og);

    let mut req_body = String::new();

    for i in 0..20 {
        let msg = format!("Hello from back {i}\n");
        ogbod.send(msg.into_bytes()).await.unwrap();

        match reqbod.next().await {
            None => continue,
            Some(Ok(reqchunk)) => req_body = format!("{req_body}{}", String::from_utf8_lossy(&reqchunk)),
            Some(Err(e)) => panic!("{e:?}"),
        }

        std::thread::sleep(std::time::Duration::from_millis(6));
    }
}
