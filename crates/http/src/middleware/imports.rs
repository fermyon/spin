use http::{header::HeaderName, HeaderMap, Request, Response};
use hyper::{body, Body};
use wit_bindgen_wasmtime::async_trait;

#[derive(Default)]
pub(crate) struct Imports {
    pub(crate) req: Option<Request<Body>>,
    pub(crate) resp: Option<Response<Body>>,
}

impl Imports {
    fn req(&mut self) -> &mut Request<Body> {
        self.req.get_or_insert_with(Default::default)
    }

    fn resp(&mut self) -> &mut Response<Body> {
        self.resp.get_or_insert_with(Default::default)
    }

    fn headers(&mut self, selector: ReqOrResp) -> &mut HeaderMap {
        match selector {
            ReqOrResp::Req => self.req().headers_mut(),
            ReqOrResp::Resp => self.resp().headers_mut(),
        }
    }

    fn body(&mut self, selector: ReqOrResp) -> &mut Body {
        match selector {
            ReqOrResp::Req => self.req().body_mut(),
            ReqOrResp::Resp => self.resp().body_mut(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum ReqOrResp {
    Req,
    Resp,
}

#[async_trait]
impl super::spin_http_middleware_imports::SpinHttpMiddlewareImports for Imports {
    type Body = ReqOrResp;
    type Headers = ReqOrResp;
    type InterceptedRequest = ();
    type InterceptedResponse = ();

    // In the middleware context the request/response resources are effectively
    // singletons, so they don't need "real" handles.
    async fn request(&mut self) -> Self::InterceptedRequest {}
    async fn response(&mut self) -> Self::InterceptedResponse {}

    async fn intercepted_request_method(&mut self, _req: &Self::InterceptedRequest) -> String {
        self.req().method().to_string()
    }

    async fn intercepted_request_set_method(
        &mut self,
        _req: &Self::InterceptedRequest,
        method: &str,
    ) {
        // TODO(lann): is panicing on invalid method OK here?
        *self.req().method_mut() = method.try_into().unwrap();
    }

    async fn intercepted_request_url(&mut self, _req: &Self::InterceptedRequest) -> String {
        self.req().uri().to_string()
    }

    async fn intercepted_request_set_url(&mut self, _req: &Self::InterceptedRequest, uri: &str) {
        // TODO(lann): is panicing on invalid URI OK here?
        *self.req().uri_mut() = uri.parse().unwrap()
    }

    async fn intercepted_request_headers(
        &mut self,
        _req: &Self::InterceptedRequest,
    ) -> Self::Headers {
        ReqOrResp::Req
    }

    async fn intercepted_request_body(&mut self, _req: &Self::InterceptedRequest) -> Self::Body {
        ReqOrResp::Req
    }

    async fn intercepted_response_status(&mut self, _req: &Self::InterceptedResponse) -> u16 {
        self.resp().status().into()
    }

    async fn intercepted_response_set_status(
        &mut self,
        _resp: &Self::InterceptedResponse,
        status: u16,
    ) {
        // TODO(lann): is panicing on invalid status OK here?
        *self.resp().status_mut() = status.try_into().unwrap();
    }

    async fn intercepted_response_headers(
        &mut self,
        _resp: &Self::InterceptedResponse,
    ) -> Self::Headers {
        ReqOrResp::Resp
    }

    async fn intercepted_response_body(&mut self, _resp: &Self::InterceptedResponse) -> Self::Body {
        ReqOrResp::Resp
    }

    // TODO(lann): think about non-utf8 header values (?)

    async fn headers_get(&mut self, selector: &Self::Headers, name: &str) -> Option<String> {
        self.headers(*selector)
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    }

    // TODO(lann): something to get multi-valued headers, e.g. headers_get_all

    async fn headers_keys(&mut self, selector: &Self::Headers) -> Vec<String> {
        self.headers(*selector)
            .keys()
            .map(|key| key.to_string())
            .collect()
    }

    async fn headers_set(&mut self, selector: &Self::Headers, name: &str, value: &str) {
        // TODO(lann): is panicing OK here?
        self.headers(*selector).insert(
            name.parse::<HeaderName>().unwrap(),
            value.try_into().unwrap(),
        );
    }

    async fn headers_append(&mut self, selector: &Self::Headers, name: &str, value: &str) {
        // TODO(lann): is panicing OK here?
        self.headers(*selector).append(
            name.parse::<HeaderName>().unwrap(),
            value.try_into().unwrap(),
        );
    }

    async fn headers_delete(&mut self, selector: &Self::Headers, name: &str) {
        self.headers(*selector)
            .remove(name.parse::<HeaderName>().unwrap());
    }

    async fn body_get(&mut self, selector: &Self::Body) -> Vec<u8> {
        let body = self.body(*selector);
        let bytes = body::to_bytes(body).await.unwrap().to_vec();
        *self.body(*selector) = bytes.clone().into();
        bytes
    }

    async fn body_set(&mut self, selector: &Self::Body, bytes: &[u8]) {
        *self.body(*selector) = bytes.to_vec().into();
    }
}
