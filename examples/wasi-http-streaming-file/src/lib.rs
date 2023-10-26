use std::{fs::File, io::Read};

use futures::SinkExt;
use spin_sdk::{
    http::{Headers, IncomingRequest, OutgoingResponse, ResponseOutparam},
    http_component,
};

const CHUNK_SIZE: usize = 1024 * 1024; // 1 MB

#[http_component]
async fn handler(_req: IncomingRequest, res: ResponseOutparam) -> anyhow::Result<()> {
    let response = OutgoingResponse::new(
        200,
        &Headers::new(&[(
            "content-type".to_string(),
            b"application/octet-stream".to_vec(),
        )]),
    );

    let mut body = response.take_body();

    ResponseOutparam::set(res, Ok(response));
    let mut file =
        File::open("target/wasm32-wasi/release/wasi_http_rust_streaming_outgoing_body.wasm")?;

    let mut buffer = vec![0; CHUNK_SIZE];

    loop {
        let bytes_read = file.read(&mut buffer[..])?;
        if bytes_read == 0 {
            break;
        }

        let data = &buffer[..bytes_read];
        body.send(data.to_vec()).await?;
        println!("sent {} bytes", data.len());
    }
    Ok(())
}
