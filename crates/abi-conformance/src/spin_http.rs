use super::Context;
use anyhow::ensure;
use wasmtime::{InstancePre, Store};

pub use spin_http::{Method, Request, SpinHttp, SpinHttpData};

wit_bindgen_wasmtime::import!("../../wit/ephemeral/spin-http.wit");

pub(super) fn test(store: &mut Store<Context>, pre: &InstancePre<Context>) -> Result<(), String> {
    super::run(|| {
        let instance = &pre.instantiate(&mut *store)?;
        let handle = SpinHttp::new(&mut *store, instance, |context| &mut context.spin_http)?;
        let response = handle.handle_http_request(
            store,
            Request {
                method: Method::Post,
                uri: "/foo",
                headers: &[("foo", "bar")],
                params: &[],
                body: Some(b"Hello, SpinHttp!"),
            },
        )?;

        ensure!(
            response.status == 200,
            "expected response status 200, got {}",
            response.status
        );

        ensure!(
            response.headers == Some(vec![("lorem".to_owned(), "ipsum".to_owned())]),
            "expected a single response header, \"lorem: ipsum\", got {:?}",
            response.headers
        );

        let expected_body = "dolor sit amet";

        ensure!(
            response.body == Some(expected_body.as_bytes().to_vec()),
            "expected a response body containing the string {expected_body:?}, got {:?}",
            response
                .body
                .as_ref()
                .map(|body| String::from_utf8_lossy(body))
        );

        Ok(())
    })
}
