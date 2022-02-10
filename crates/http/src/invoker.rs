use crate::{spin_http::SpinHttp, ExecutionContext};
use anyhow::Result;
use spin_config::CoreComponent;
pub use spin_invoke::add_to_linker;
use spin_invoke::{Request, Response, SpinInvoke};
use std::sync::Arc;
use tracing::log;

wit_bindgen_wasmtime::export!("wit/ephemeral/spin-invoke.wit");
wit_bindgen_wasmtime::import!("wit/ephemeral/spin-http.wit");

#[derive(Clone, Default)]
pub struct InternalInvoker {
    pub app: spin_config::Configuration<CoreComponent>,
    pub engine: Arc<ExecutionContext>,
}

impl SpinInvoke for InternalInvoker {
    fn invoke_http(&mut self, id: &str, req: Request) -> Response {
        self.http(id, req).expect("cannot invoke HTTP component")
    }
}

impl InternalInvoker {
    pub(crate) fn http(&mut self, id: &str, req: Request) -> Result<Response> {
        log::info!("Sending an internal request to component {}", id);
        let (mut store, instance) = self.engine.prepare_component(&id.to_string(), None)?;
        let engine = SpinHttp::new(&mut store, &instance, |host| {
            &mut host.data.as_mut().unwrap().http
        })?;

        let req = crate::spin_http::Request {
            method: Self::method(req.method),
            uri: spin_http::Uri::from(req.uri),
            headers: spin_http::HeadersParam::from(&req.headers),
            params: spin_http::Params::from(&req.params),
            body: req.body,
        };

        Ok(Response::from(engine.handler(&mut store, req)?))
    }
}

impl InternalInvoker {
    fn method(m: spin_invoke::Method) -> crate::spin_http::Method {
        match m {
            spin_invoke::Method::Get => crate::spin_http::Method::Get,
            spin_invoke::Method::Post => crate::spin_http::Method::Post,
            spin_invoke::Method::Put => crate::spin_http::Method::Put,
            spin_invoke::Method::Delete => crate::spin_http::Method::Delete,
            spin_invoke::Method::Patch => crate::spin_http::Method::Patch,
            spin_invoke::Method::Head => crate::spin_http::Method::Head,
            spin_invoke::Method::Options => crate::spin_http::Method::Options,
        }
    }
}

impl From<crate::spin_http::Response> for Response {
    fn from(res: crate::spin_http::Response) -> Self {
        let headers = match &res.headers {
            Some(h) => Some(h.clone()),
            None => None,
        };

        Response {
            status: res.status,
            headers,
            body: res.body,
        }
    }
}
