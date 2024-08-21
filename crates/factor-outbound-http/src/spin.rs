use spin_world::{
    async_trait,
    v1::http,
    v1::http_types::{self, HttpError, Request, Response},
};

#[async_trait]
impl http::Host for crate::InstanceState {
    async fn send_request(&mut self, req: Request) -> Result<Response, HttpError> {
        // FIXME(lann): This is all just a stub to test allowed_outbound_hosts
        match self.allowed_hosts.check_url(&req.uri, "https").await {
            Ok(true) => (),
            _ => {
                return Err(HttpError::DestinationNotAllowed);
            }
        }
        Ok(Response {
            status: 200,
            headers: None,
            body: Some(b"test response".into()),
        })
    }
}

impl http_types::Host for crate::InstanceState {
    fn convert_http_error(&mut self, err: HttpError) -> anyhow::Result<HttpError> {
        Ok(err)
    }
}
