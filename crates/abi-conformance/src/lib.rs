//! Spin ABI Conformance Test Suite
//!
//! This crate provides a suite of tests to check a given SDK or language integration's implementation of Spin
//! functions.  It is intended for use by language integrators and SDK authors to verify that their integrations
//! and SDKs work correctly with the Spin ABIs.  It is not intended for Spin _application_ development, since it
//! requires a module written specifically to behave as expected by this suite, whereas a given application will
//! have its own expected behaviors which can only be verified by tests specific to that application.
//!
//! The suite may be run via the [`test()`] function, which accepts a [`wasmtime::Module`] and a [`Config`] and
//! returns a [`Report`] which details which tests succeeded and which failed.  The definition of success in this
//! context depends on whether the test is for a function implemented by the guest (e.g. triggers) or by the host
//! (e.g. outbound requests).
//!
//! - For a guest-implemented function, the host will call the function and assert the result matches what is
//! expected (see [`Report::http_trigger`] for an example).
//!
//! - For a host-implemented function, the host will call a guest-implemented function according to the specified
//! [`InvocationStyle`] with a set of arguments indicating which host function to call and with what arguments.
//! The host then asserts that host function was indeed called with the expected arguments (see
//! [`Report::outbound_http`] for an example).

use anyhow::{Context as _, Result};
use outbound_http::OutboundHttp;
use outbound_pg::OutboundPg;
use outbound_redis::OutboundRedis;
use serde::{Deserialize, Serialize};
use spin_config::SpinConfig;
use spin_http::{Method, Request, SpinHttp, SpinHttpData};
use spin_redis::SpinRedisData;
use std::str;
use wasi_common::{pipe::WritePipe, WasiCtx};
use wasmtime::{InstancePre, Linker, Module, Store};
use wasmtime_wasi::WasiCtxBuilder;

pub use outbound_pg::PgReport;
pub use outbound_redis::RedisReport;
pub use wasi::WasiReport;

mod outbound_http;
mod outbound_pg;
mod outbound_redis;
mod spin_config;
mod spin_http;
mod spin_redis;
mod wasi;

/// The invocation style to use when the host asks the guest to call a host-implemented function
#[derive(Copy, Clone, Default, Deserialize)]
pub enum InvocationStyle {
    /// The host should call into the guest using WASI's `_start` function, passing arguments as CLI parameters.
    ///
    /// This is the default if no value is specified.
    #[default]
    Command,

    /// The host should call into the guest using spin-http.wit's `handle-http-request` function, passing arguments
    /// via the request body as a JSON array of strings.
    HttpTrigger,
}

/// Configuration options for the [`test()`] function
#[derive(Default, Deserialize)]
pub struct Config {
    /// The invocation style to use when the host asks the guest to call a host-implemented function
    #[serde(default)]
    pub invocation_style: InvocationStyle,
}

/// Report of which tests succeeded or failed
///
/// These results fall into either of two categories:
///
/// - Guest-implemented exports which behave as prescribed by the test (e.g. `http_trigger` and `redis_trigger`)
///
/// - Host-implemented imports which are called by the guest with the arguments specified by the host
/// (e.g. `outbound_http`)
#[derive(Serialize)]
pub struct Report {
    /// Result of the Spin HTTP trigger test
    ///
    /// The guest module should expect a call to `handle-http-request` with a POST request to "/foo" containing a
    /// single header "foo: bar" and a UTF-8 string body "Hello, SpinHttp!" and return a 200 OK response that
    /// includes a single header "lorem: ipsum" and a UTF-8 string body "dolor sit amet".
    pub http_trigger: Result<(), String>,

    /// Result of the Spin Redis trigger test
    ///
    /// The guest module should expect a call to `handle-redis-message` with the text "Hello, SpinRedis!" and
    /// return `ok(unit)` as the result.
    pub redis_trigger: Result<(), String>,

    /// Result of the Spin config test
    ///
    /// The guest module should expect a call according to [`InvocationStyle`] with \["config", "foo"\] as
    /// arguments.  The module should call the host-implemented `spin-config::get-config` function with "foo" as
    /// the argument and expect `ok("bar")` as the result.  The host will assert that said function is called
    /// exactly once with the expected argument.
    pub config: Result<(), String>,

    /// Result of the Spin outbound HTTP test
    ///
    /// The guest module should expect a call according to [`InvocationStyle`] with \["outbound-http",
    /// "http://127.0.0.1/test"\] as arguments.  The module should call the host-implemented
    /// `wasi-outbound-http::request` function with a GET request for the URL "http://127.0.0.1/test" with no
    /// headers, params, or body, and expect `ok({ status: 200, headers: none, body: some("Jabberwocky"))` as the
    /// result.  The host will assert that said function is called exactly once with the specified argument.
    pub outbound_http: Result<(), String>,

    /// Results of the Spin outbound Redis tests
    ///
    /// See [`RedisReport`] for details.
    pub outbound_redis: RedisReport,

    /// Results of the Spin outbound PostgreSQL tests
    ///
    /// See [`PgReport`] for details.
    pub outbound_pg: PgReport,

    /// Results of the WASI tests
    ///
    /// See [`WasiReport`] for details.
    pub wasi: WasiReport,
}

/// Run a test for each Spin-related function the specified `module` imports or exports, returning the results as a
/// [`Report`].
///
/// See the fields of [`Report`] and the structs from which it is composed for descriptions of each test.
pub fn test(module: &Module, config: Config) -> Result<Report> {
    let mut store = Store::new(
        module.engine(),
        Context {
            config,
            wasi: WasiCtxBuilder::new().arg("<wasm module>")?.build(),
            outbound_http: OutboundHttp::default(),
            outbound_redis: OutboundRedis::default(),
            outbound_pg: OutboundPg::default(),
            spin_http: SpinHttpData {},
            spin_redis: SpinRedisData {},
            spin_config: SpinConfig::default(),
        },
    );

    let mut linker = Linker::<Context>::new(module.engine());
    wasmtime_wasi::add_to_linker(&mut linker, |context| &mut context.wasi)?;
    outbound_http::add_to_linker(&mut linker, |context| &mut context.outbound_http)?;
    outbound_redis::add_to_linker(&mut linker, |context| &mut context.outbound_redis)?;
    outbound_pg::add_to_linker(&mut linker, |context| &mut context.outbound_pg)?;
    spin_config::add_to_linker(&mut linker, |context| &mut context.spin_config)?;

    let pre = linker.instantiate_pre(&mut store, module)?;

    Ok(Report {
        http_trigger: spin_http::test(&mut store, &pre),

        redis_trigger: spin_redis::test(&mut store, &pre),

        config: spin_config::test(&mut store, &pre),

        outbound_http: outbound_http::test(&mut store, &pre),

        outbound_redis: outbound_redis::test(&mut store, &pre)?,

        outbound_pg: outbound_pg::test(&mut store, &pre)?,

        wasi: wasi::test(&mut store, &pre)?,
    })
}

struct Context {
    config: Config,
    wasi: WasiCtx,
    outbound_http: OutboundHttp,
    outbound_redis: OutboundRedis,
    outbound_pg: OutboundPg,
    spin_http: SpinHttpData,
    spin_redis: SpinRedisData,
    spin_config: SpinConfig,
}

fn run(fun: impl FnOnce() -> Result<()>) -> Result<(), String> {
    fun().map_err(|e| format!("{e:?}"))
}

fn run_command(
    store: &mut Store<Context>,
    pre: &InstancePre<Context>,
    arguments: &[&str],
    fun: impl FnOnce(&mut Store<Context>) -> Result<()>,
) -> Result<(), String> {
    run(|| {
        let stderr = WritePipe::new_in_memory();
        store.data_mut().wasi.set_stderr(Box::new(stderr.clone()));

        let instance = &pre.instantiate(&mut *store)?;

        let result = match store.data().config.invocation_style {
            InvocationStyle::HttpTrigger => {
                let handle =
                    SpinHttp::new(&mut *store, instance, |context| &mut context.spin_http)?;

                handle
                    .handle_http_request(
                        &mut *store,
                        Request {
                            method: Method::Post,
                            uri: "/",
                            headers: &[],
                            params: &[],
                            client_addr: "",
                            body: Some(&serde_json::to_vec(arguments)?),
                        },
                    )
                    .map(drop) // Ignore the response and make this a `Result<(), Trap>` to match the `_start` case
                               // below
            }

            InvocationStyle::Command => {
                for argument in arguments {
                    store.data_mut().wasi.push_arg(argument)?;
                }

                instance
                    .get_typed_func::<(), ()>(&mut *store, "_start")?
                    .call(&mut *store, ())
            }
        };

        // Reset `Context::wasi` so the next test has a clean slate and also to ensure there are no more references
        // to the `stderr` pipe, ensuring `try_into_inner` succeeds below.  This is also needed in case the caller
        // attached its own pipes for e.g. stdin and/or stdout and expects exclusive ownership once we return.
        store.data_mut().wasi = WasiCtxBuilder::new().arg("<wasm module>")?.build();

        result.with_context(|| {
            String::from_utf8_lossy(&stderr.try_into_inner().unwrap().into_inner()).into_owned()
        })?;

        fun(store)
    })
}
