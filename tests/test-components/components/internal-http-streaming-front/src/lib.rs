use anyhow::anyhow;
use futures::{SinkExt, TryStreamExt};
use helper::{ensure, ensure_eq, ensure_ok};
use spin_sdk::{
    http::{IntoResponse, Request},
    http_component,
};

/// Send an HTTP request and return the response.
#[http_component]
async fn handle_front(req: Request) -> anyhow::Result<impl IntoResponse> {
    handle_front_impl(req).await.map_err(|e| anyhow!(e))
}

async fn handle_front_impl(_req: Request) -> Result<impl IntoResponse, String> {
    let out_req = spin_sdk::http::OutgoingRequest::new(
        spin_sdk::http::Fields::new()
    );
    out_req.set_method(&spin_sdk::http::Method::Post).unwrap();
    out_req.set_authority(Some("back-streaming.spin.internal")).unwrap();
    out_req.set_scheme(Some(&spin_sdk::http::Scheme::Http)).unwrap();
    out_req.set_path_with_query(Some("/")).unwrap();

    let mut obod = out_req.take_body();

    let resp: spin_sdk::http::IncomingResponse = ensure_ok!(spin_sdk::http::send(out_req).await);

    ensure_eq!(200, resp.status());

    let resp_stm = resp.take_body_stream();

    let resp_fut = resp_stm.try_fold(vec![], |mut acc, mut item| async move {
        acc.append(&mut item);
        Ok(acc)
    });

    let send_fut = async move {
        for i in 0..10 {
            let msg = format!("Hello from front {i}");
            ensure_ok!(obod.send(msg.into_bytes()).await);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        obod.flush().await.unwrap();
        obod.close().await.unwrap();
        Ok(())
    };

    let (send, resp_body) = futures::future::join(send_fut, resp_fut).await;

    ensure_ok!(send);
    let resp_body = ensure_ok!(resp_body);

    let resp_body = String::from_utf8_lossy(&resp_body);
    ensure!(resp_body.contains("Hello from back 0"));
    ensure!(resp_body.contains("Hello from back 19"));

    Ok(spin_sdk::http::Response::new(200, ""))
}
