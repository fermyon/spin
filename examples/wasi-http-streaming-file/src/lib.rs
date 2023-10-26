use std::{fs::File, io::Read};

use anyhow::Result;
use futures::SinkExt;
use spin_sdk::{
    http::{Headers, IncomingRequest, OutgoingResponse, ResponseOutparam},
    http_component,
};

const CHUNK_SIZE: usize = 1024 * 1024; // 1 MB

#[http_component]
async fn handler(req: IncomingRequest, res: ResponseOutparam) {
    stream_file(req, res).await.unwrap();
}

async fn stream_file(_req: IncomingRequest, res: ResponseOutparam) -> Result<()> {
    let response = OutgoingResponse::new(
        200,
        &Headers::new(&[(
            "content-type".to_string(),
            b"application/octet-stream".to_vec(),
        )]),
    );

    let mut body = response.take_body();
    res.set(response);

    let mut file = File::open("target/wasm32-wasi/release/spin_wasi_http_streaming_file.wasm")?;

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
