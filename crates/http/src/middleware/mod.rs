#![deny(missing_docs)]

use anyhow::{bail, Context, Result};
use hyper::{Body, Request, Response};
use tracing::log;
use wasmtime::{Instance, Linker, Store};

wit_bindgen_wasmtime::export!({paths: ["wit/ephemeral/spin-http-middleware-imports.wit"], async: *});
wit_bindgen_wasmtime::import!({paths: ["wit/ephemeral/spin-http-middleware-request.wit"], async: *});
wit_bindgen_wasmtime::import!({paths: ["wit/ephemeral/spin-http-middleware-response.wit"], async: *});
use crate::{ExecutionContext, RuntimeContext};

use self::{
    spin_http_middleware_imports::SpinHttpMiddlewareImportsTables,
    spin_http_middleware_request::{InterceptRequestAction, SpinHttpMiddlewareRequestData},
    spin_http_middleware_response::SpinHttpMiddlewareResponseData,
};

mod imports;
use self::imports::Imports;

const INTERCEPT_REQUEST_NAME: &str = "intercept-request";
const INTERCEPT_RESPONSE_NAME: &str = "intercept-response";

#[derive(Default)]
pub struct MiddlewareData {
    imports: Imports,
    tables: SpinHttpMiddlewareImportsTables<Imports>,
    req_data: SpinHttpMiddlewareRequestData,
    resp_data: SpinHttpMiddlewareResponseData,
}
pub(crate) struct MiddlewaresStack {
    middlewares: Vec<MiddlewareInstance>,
}

impl MiddlewaresStack {
    pub(crate) fn create<'a>(
        engine: &ExecutionContext,
        middleware_ids: impl IntoIterator<Item = &'a String>,
    ) -> Result<Self> {
        let mut middlewares = vec![];
        for component_id in middleware_ids {
            middlewares.push(MiddlewareInstance::prepare(engine, component_id)?);
        }
        Ok(Self { middlewares })
    }

    pub(crate) async fn execute_request_middlewares(
        &mut self,
        mut req: Request<Body>,
    ) -> Result<RequestMiddlewareResult> {
        for (idx, middleware) in self.middlewares.iter_mut().enumerate() {
            req = match middleware
                .intercept_request(req)
                .await
                .with_context(|| format!("request middleware {}", middleware.id))?
            {
                RequestMiddlewareResult::Next(req) => req,
                RequestMiddlewareResult::Stop(mut resp) => {
                    // Unwind previous middlewares
                    if idx > 0 {
                        resp = self._execute_response_middlewares(resp, idx).await?;
                    }
                    return Ok(RequestMiddlewareResult::Stop(resp));
                }
            }
        }
        Ok(RequestMiddlewareResult::Next(req))
    }

    pub(crate) async fn execute_response_middlewares(
        &mut self,
        resp: Response<Body>,
    ) -> Result<Response<Body>> {
        self._execute_response_middlewares(resp, self.middlewares.len())
            .await
    }

    async fn _execute_response_middlewares(
        &mut self,
        mut resp: Response<Body>,
        end: usize,
    ) -> Result<Response<Body>> {
        for middleware in self.middlewares[..end].iter_mut().rev() {
            resp = middleware
                .intercept_response(resp)
                .await
                .with_context(|| format!("response middleware {}", middleware.id))?;
        }
        Ok(resp)
    }
}

struct MiddlewareInstance {
    id: String,
    store: Store<RuntimeContext>,
    instance: Instance,
    is_request_interceptor: bool,
    is_response_interceptor: bool,
}

impl MiddlewareInstance {
    fn prepare(engine: &ExecutionContext, component_id: impl Into<String>) -> Result<Self> {
        let id = component_id.into();
        let (mut store, instance) =
            engine.prepare_component(id.as_str(), Some(Default::default()), None, None, None)?;
        let is_request_interceptor = instance
            .get_export(&mut store, INTERCEPT_REQUEST_NAME)
            .is_some();
        let is_response_interceptor = instance
            .get_export(&mut store, INTERCEPT_RESPONSE_NAME)
            .is_some();
        if !is_request_interceptor && !is_response_interceptor {
            bail!(
                "middleware {} has no {} or {} function",
                &id,
                INTERCEPT_REQUEST_NAME,
                INTERCEPT_RESPONSE_NAME
            );
        }
        Ok(Self {
            id,
            store,
            instance,
            is_request_interceptor,
            is_response_interceptor,
        })
    }

    async fn intercept_request(&mut self, req: Request<Body>) -> Result<RequestMiddlewareResult> {
        if !self.is_request_interceptor {
            return Ok(RequestMiddlewareResult::Next(req));
        }

        // Insert Request into middleware context
        self.store.middleware_data().imports.req = Some(req);

        // Execute request middleware
        let action = spin_http_middleware_request::SpinHttpMiddlewareRequest::new(
            &mut self.store,
            &self.instance,
            |ctx| &mut ctx.middleware_data().req_data,
        )?
        .intercept_request(&mut self.store)
        .await?;

        // Take Request and (maybe) Response out of middleware context
        let imports = &mut self.store.middleware_data().imports;
        let req = imports.req.take().unwrap();
        let resp = imports.resp.take();

        match action {
            InterceptRequestAction::Next => {
                if resp.is_some() {
                    log::warn!(
                        "Request middleware {} returned 'next' but response was initialized",
                        self.id
                    );
                }
                Ok(RequestMiddlewareResult::Next(req))
            }
            InterceptRequestAction::Stop => {
                if let Some(resp) = resp {
                    Ok(RequestMiddlewareResult::Stop(resp))
                } else {
                    bail!("middleware returned 'stop' but didn't initialize response")
                }
            }
        }
    }

    async fn intercept_response(&mut self, resp: Response<Body>) -> Result<Response<Body>> {
        if !self.is_response_interceptor {
            return Ok(resp);
        }

        // Insert Response into middleware context
        self.store.middleware_data().imports.resp = Some(resp);

        // Execute response middleware
        spin_http_middleware_response::SpinHttpMiddlewareResponse::new(
            &mut self.store,
            &self.instance,
            |ctx| &mut ctx.data.as_mut().unwrap().middleware.resp_data,
        )?
        .intercept_response(&mut self.store)
        .await?;

        // Take Response back out of middleware context
        let resp = self.store.middleware_data().imports.resp.take().unwrap();

        Ok(resp)
    }
}

pub(crate) enum RequestMiddlewareResult {
    Next(Request<Body>),
    Stop(Response<Body>),
}

pub(crate) fn add_middleware_to_linker(linker: &mut Linker<RuntimeContext>) -> Result<()> {
    spin_http_middleware_imports::add_to_linker(linker, |ctx: &mut RuntimeContext| {
        let data = ctx.data.as_mut().unwrap();
        (&mut data.middleware.imports, &mut data.middleware.tables)
    })
}

trait MiddlewareDataMut {
    fn middleware_data(&mut self) -> &mut MiddlewareData;
}

impl MiddlewareDataMut for RuntimeContext {
    fn middleware_data(&mut self) -> &mut MiddlewareData {
        &mut self.data.as_mut().unwrap().middleware
    }
}

impl MiddlewareDataMut for Store<RuntimeContext> {
    fn middleware_data(&mut self) -> &mut MiddlewareData {
        self.data_mut().middleware_data()
    }
}
