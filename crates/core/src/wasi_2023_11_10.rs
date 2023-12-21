#![doc(hidden)] // internal implementation detail used in tests and spin-trigger

use super::wasi_2023_10_18::convert;
use anyhow::Result;
use wasmtime::component::{Linker, Resource};
use wasmtime_wasi::preview2::WasiView;
use wasmtime_wasi_http::WasiHttpView;

mod latest {
    pub use wasmtime_wasi::preview2::bindings::wasi::*;
    pub mod http {
        pub use wasmtime_wasi_http::bindings::wasi::http::*;
    }
}

wasmtime::component::bindgen!({
    path: "../../wit-2023-11-10",
    interfaces: r#"
        include wasi:http/proxy@0.2.0-rc-2023-11-10;

        // NB: this is handling the historical behavior where Spin supported
        // more than "just" this snaphsot of the proxy world but additionally
        // other CLI-related interfaces.
        import wasi:cli/environment@0.2.0-rc-2023-11-10;
        import wasi:cli/exit@0.2.0-rc-2023-11-10;
        import wasi:cli/stdin@0.2.0-rc-2023-11-10;
        import wasi:cli/stdout@0.2.0-rc-2023-11-10;
        import wasi:cli/stderr@0.2.0-rc-2023-11-10;
        import wasi:cli/terminal-input@0.2.0-rc-2023-11-10;
        import wasi:cli/terminal-output@0.2.0-rc-2023-11-10;
        import wasi:cli/terminal-stdin@0.2.0-rc-2023-11-10;
        import wasi:cli/terminal-stdout@0.2.0-rc-2023-11-10;
        import wasi:cli/terminal-stderr@0.2.0-rc-2023-11-10;
    "#,
    async: {
        only_imports: []
    },
    with: {
        "wasi:io/poll/pollable": latest::io::poll::Pollable,
        "wasi:io/streams/input-stream": latest::io::streams::InputStream,
        "wasi:io/streams/output-stream": latest::io::streams::OutputStream,
        "wasi:io/error/error": latest::io::error::Error,
        "wasi:cli/terminal-input/terminal-input": latest::cli::terminal_input::TerminalInput,
        "wasi:cli/terminal-output/terminal-output": latest::cli::terminal_output::TerminalOutput,
        "wasi:http/types/incoming-response": latest::http::types::IncomingResponse,
        "wasi:http/types/incoming-request": latest::http::types::IncomingRequest,
        "wasi:http/types/incoming-body": latest::http::types::IncomingBody,
        "wasi:http/types/outgoing-response": latest::http::types::OutgoingResponse,
        "wasi:http/types/outgoing-request": latest::http::types::OutgoingRequest,
        "wasi:http/types/outgoing-body": latest::http::types::OutgoingBody,
        "wasi:http/types/fields": latest::http::types::Fields,
        "wasi:http/types/response-outparam": latest::http::types::ResponseOutparam,
        "wasi:http/types/future-incoming-response": latest::http::types::FutureIncomingResponse,
        "wasi:http/types/future-trailers": latest::http::types::FutureTrailers,
        "wasi:http/types/request-options": latest::http::types::RequestOptions,
    },
});

use wasi::cli::terminal_input::TerminalInput;
use wasi::cli::terminal_output::TerminalOutput;
use wasi::clocks::wall_clock::Datetime;
use wasi::http::types::{
    DnsErrorPayload, ErrorCode as HttpErrorCode, FieldSizePayload, Fields, FutureIncomingResponse,
    FutureTrailers, HeaderError, Headers, IncomingBody, IncomingRequest, IncomingResponse, Method,
    OutgoingBody, OutgoingRequest, OutgoingResponse, RequestOptions, ResponseOutparam, Scheme,
    StatusCode, TlsAlertReceivedPayload, Trailers,
};
use wasi::io::poll::Pollable;
use wasi::io::streams::{Error, InputStream, OutputStream};

pub fn add_to_linker<T>(linker: &mut Linker<T>) -> Result<()>
where
    T: WasiView + WasiHttpView,
{
    // interfaces from the "command" world
    wasi::cli::exit::add_to_linker(linker, |t| t)?;
    wasi::cli::environment::add_to_linker(linker, |t| t)?;
    wasi::cli::stdin::add_to_linker(linker, |t| t)?;
    wasi::cli::stdout::add_to_linker(linker, |t| t)?;
    wasi::cli::stderr::add_to_linker(linker, |t| t)?;
    wasi::cli::terminal_input::add_to_linker(linker, |t| t)?;
    wasi::cli::terminal_output::add_to_linker(linker, |t| t)?;
    wasi::cli::terminal_stdin::add_to_linker(linker, |t| t)?;
    wasi::cli::terminal_stdout::add_to_linker(linker, |t| t)?;
    wasi::cli::terminal_stderr::add_to_linker(linker, |t| t)?;

    wasi::http::types::add_to_linker(linker, |t| t)?;
    wasi::http::outgoing_handler::add_to_linker(linker, |t| t)?;
    Ok(())
}

impl<T> wasi::cli::exit::Host for T
where
    T: WasiView,
{
    fn exit(&mut self, status: Result<(), ()>) -> wasmtime::Result<()> {
        <T as latest::cli::exit::Host>::exit(self, status)
    }
}

impl<T> wasi::cli::environment::Host for T
where
    T: WasiView,
{
    fn get_environment(&mut self) -> wasmtime::Result<Vec<(String, String)>> {
        <T as latest::cli::environment::Host>::get_environment(self)
    }

    fn get_arguments(&mut self) -> wasmtime::Result<Vec<String>> {
        <T as latest::cli::environment::Host>::get_arguments(self)
    }

    fn initial_cwd(&mut self) -> wasmtime::Result<Option<String>> {
        <T as latest::cli::environment::Host>::initial_cwd(self)
    }
}

impl<T> wasi::cli::stdin::Host for T
where
    T: WasiView,
{
    fn get_stdin(&mut self) -> wasmtime::Result<Resource<InputStream>> {
        <T as latest::cli::stdin::Host>::get_stdin(self)
    }
}

impl<T> wasi::cli::stdout::Host for T
where
    T: WasiView,
{
    fn get_stdout(&mut self) -> wasmtime::Result<Resource<OutputStream>> {
        <T as latest::cli::stdout::Host>::get_stdout(self)
    }
}

impl<T> wasi::cli::stderr::Host for T
where
    T: WasiView,
{
    fn get_stderr(&mut self) -> wasmtime::Result<Resource<OutputStream>> {
        <T as latest::cli::stderr::Host>::get_stderr(self)
    }
}

impl<T> wasi::cli::terminal_stdin::Host for T
where
    T: WasiView,
{
    fn get_terminal_stdin(&mut self) -> wasmtime::Result<Option<Resource<TerminalInput>>> {
        <T as latest::cli::terminal_stdin::Host>::get_terminal_stdin(self)
    }
}

impl<T> wasi::cli::terminal_stdout::Host for T
where
    T: WasiView,
{
    fn get_terminal_stdout(&mut self) -> wasmtime::Result<Option<Resource<TerminalOutput>>> {
        <T as latest::cli::terminal_stdout::Host>::get_terminal_stdout(self)
    }
}

impl<T> wasi::cli::terminal_stderr::Host for T
where
    T: WasiView,
{
    fn get_terminal_stderr(&mut self) -> wasmtime::Result<Option<Resource<TerminalOutput>>> {
        <T as latest::cli::terminal_stderr::Host>::get_terminal_stderr(self)
    }
}

impl<T> wasi::cli::terminal_input::Host for T where T: WasiView {}

impl<T> wasi::cli::terminal_input::HostTerminalInput for T
where
    T: WasiView,
{
    fn drop(&mut self, rep: Resource<TerminalInput>) -> wasmtime::Result<()> {
        <T as latest::cli::terminal_input::HostTerminalInput>::drop(self, rep)
    }
}

impl<T> wasi::cli::terminal_output::Host for T where T: WasiView {}

impl<T> wasi::cli::terminal_output::HostTerminalOutput for T
where
    T: WasiView,
{
    fn drop(&mut self, rep: Resource<TerminalOutput>) -> wasmtime::Result<()> {
        <T as latest::cli::terminal_output::HostTerminalOutput>::drop(self, rep)
    }
}

impl<T> wasi::http::types::Host for T
where
    T: WasiHttpView,
{
    fn http_error_code(
        &mut self,
        error: Resource<Error>,
    ) -> wasmtime::Result<Option<HttpErrorCode>> {
        <T as latest::http::types::Host>::http_error_code(self, error).map(|e| e.map(|e| e.into()))
    }
}

impl<T> wasi::http::types::HostRequestOptions for T
where
    T: WasiHttpView,
{
    fn new(&mut self) -> wasmtime::Result<Resource<RequestOptions>> {
        <T as latest::http::types::HostRequestOptions>::new(self)
    }

    fn connect_timeout_ms(
        &mut self,
        self_: Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<u64>> {
        <T as latest::http::types::HostRequestOptions>::connect_timeout(self, self_)
    }

    fn set_connect_timeout_ms(
        &mut self,
        self_: Resource<RequestOptions>,
        duration: Option<u64>,
    ) -> wasmtime::Result<Result<(), ()>> {
        <T as latest::http::types::HostRequestOptions>::set_connect_timeout(self, self_, duration)
    }

    fn first_byte_timeout_ms(
        &mut self,
        self_: Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<u64>> {
        <T as latest::http::types::HostRequestOptions>::first_byte_timeout(self, self_)
    }

    fn set_first_byte_timeout_ms(
        &mut self,
        self_: Resource<RequestOptions>,
        duration: Option<u64>,
    ) -> wasmtime::Result<Result<(), ()>> {
        <T as latest::http::types::HostRequestOptions>::set_first_byte_timeout(
            self, self_, duration,
        )
    }

    fn between_bytes_timeout_ms(
        &mut self,
        self_: Resource<RequestOptions>,
    ) -> wasmtime::Result<Option<u64>> {
        <T as latest::http::types::HostRequestOptions>::between_bytes_timeout(self, self_)
    }

    fn set_between_bytes_timeout_ms(
        &mut self,
        self_: Resource<RequestOptions>,
        duration: Option<u64>,
    ) -> wasmtime::Result<Result<(), ()>> {
        <T as latest::http::types::HostRequestOptions>::set_between_bytes_timeout(
            self, self_, duration,
        )
    }

    fn drop(&mut self, self_: Resource<RequestOptions>) -> wasmtime::Result<()> {
        <T as latest::http::types::HostRequestOptions>::drop(self, self_)
    }
}

impl<T> wasi::http::types::HostFields for T
where
    T: WasiHttpView,
{
    fn new(&mut self) -> wasmtime::Result<Resource<Fields>> {
        <T as latest::http::types::HostFields>::new(self)
    }

    fn from_list(
        &mut self,
        entries: Vec<(String, Vec<u8>)>,
    ) -> wasmtime::Result<Result<Resource<Fields>, HeaderError>> {
        <T as latest::http::types::HostFields>::from_list(self, entries)
            .map(|r| r.map_err(|e| e.into()))
    }

    fn get(&mut self, self_: Resource<Fields>, name: String) -> wasmtime::Result<Vec<Vec<u8>>> {
        <T as latest::http::types::HostFields>::get(self, self_, name)
    }

    fn set(
        &mut self,
        self_: Resource<Fields>,
        name: String,
        value: Vec<Vec<u8>>,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        <T as latest::http::types::HostFields>::set(self, self_, name, value)
            .map(|r| r.map_err(|e| e.into()))
    }

    fn delete(
        &mut self,
        self_: Resource<Fields>,
        name: String,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        <T as latest::http::types::HostFields>::delete(self, self_, name)
            .map(|r| r.map_err(|e| e.into()))
    }

    fn append(
        &mut self,
        self_: Resource<Fields>,
        name: String,
        value: Vec<u8>,
    ) -> wasmtime::Result<Result<(), HeaderError>> {
        <T as latest::http::types::HostFields>::append(self, self_, name, value)
            .map(|r| r.map_err(|e| e.into()))
    }

    fn entries(&mut self, self_: Resource<Fields>) -> wasmtime::Result<Vec<(String, Vec<u8>)>> {
        <T as latest::http::types::HostFields>::entries(self, self_)
    }

    fn clone(&mut self, self_: Resource<Fields>) -> wasmtime::Result<Resource<Fields>> {
        <T as latest::http::types::HostFields>::clone(self, self_)
    }

    fn drop(&mut self, rep: Resource<Fields>) -> wasmtime::Result<()> {
        <T as latest::http::types::HostFields>::drop(self, rep)
    }
}

impl<T> wasi::http::types::HostIncomingRequest for T
where
    T: WasiHttpView,
{
    fn method(&mut self, self_: Resource<IncomingRequest>) -> wasmtime::Result<Method> {
        <T as latest::http::types::HostIncomingRequest>::method(self, self_).map(|e| e.into())
    }

    fn path_with_query(
        &mut self,
        self_: Resource<IncomingRequest>,
    ) -> wasmtime::Result<Option<String>> {
        <T as latest::http::types::HostIncomingRequest>::path_with_query(self, self_)
    }

    fn scheme(&mut self, self_: Resource<IncomingRequest>) -> wasmtime::Result<Option<Scheme>> {
        <T as latest::http::types::HostIncomingRequest>::scheme(self, self_)
            .map(|e| e.map(|e| e.into()))
    }

    fn authority(&mut self, self_: Resource<IncomingRequest>) -> wasmtime::Result<Option<String>> {
        <T as latest::http::types::HostIncomingRequest>::authority(self, self_)
    }

    fn headers(&mut self, self_: Resource<IncomingRequest>) -> wasmtime::Result<Resource<Headers>> {
        <T as latest::http::types::HostIncomingRequest>::headers(self, self_)
    }

    fn consume(
        &mut self,
        self_: Resource<IncomingRequest>,
    ) -> wasmtime::Result<Result<Resource<IncomingBody>, ()>> {
        <T as latest::http::types::HostIncomingRequest>::consume(self, self_)
    }

    fn drop(&mut self, rep: Resource<IncomingRequest>) -> wasmtime::Result<()> {
        <T as latest::http::types::HostIncomingRequest>::drop(self, rep)
    }
}

impl<T> wasi::http::types::HostIncomingResponse for T
where
    T: WasiHttpView,
{
    fn status(&mut self, self_: Resource<IncomingResponse>) -> wasmtime::Result<StatusCode> {
        <T as latest::http::types::HostIncomingResponse>::status(self, self_)
    }

    fn headers(
        &mut self,
        self_: Resource<IncomingResponse>,
    ) -> wasmtime::Result<Resource<Headers>> {
        <T as latest::http::types::HostIncomingResponse>::headers(self, self_)
    }

    fn consume(
        &mut self,
        self_: Resource<IncomingResponse>,
    ) -> wasmtime::Result<Result<Resource<IncomingBody>, ()>> {
        <T as latest::http::types::HostIncomingResponse>::consume(self, self_)
    }

    fn drop(&mut self, rep: Resource<IncomingResponse>) -> wasmtime::Result<()> {
        <T as latest::http::types::HostIncomingResponse>::drop(self, rep)
    }
}

impl<T> wasi::http::types::HostIncomingBody for T
where
    T: WasiHttpView,
{
    fn stream(
        &mut self,
        self_: Resource<IncomingBody>,
    ) -> wasmtime::Result<Result<Resource<InputStream>, ()>> {
        <T as latest::http::types::HostIncomingBody>::stream(self, self_)
    }

    fn finish(
        &mut self,
        this: Resource<IncomingBody>,
    ) -> wasmtime::Result<Resource<FutureTrailers>> {
        <T as latest::http::types::HostIncomingBody>::finish(self, this)
    }

    fn drop(&mut self, rep: Resource<IncomingBody>) -> wasmtime::Result<()> {
        <T as latest::http::types::HostIncomingBody>::drop(self, rep)
    }
}

impl<T> wasi::http::types::HostOutgoingRequest for T
where
    T: WasiHttpView,
{
    fn new(&mut self, headers: Resource<Headers>) -> wasmtime::Result<Resource<OutgoingRequest>> {
        <T as latest::http::types::HostOutgoingRequest>::new(self, headers)
    }

    fn method(&mut self, self_: Resource<OutgoingRequest>) -> wasmtime::Result<Method> {
        <T as latest::http::types::HostOutgoingRequest>::method(self, self_).map(|m| m.into())
    }

    fn set_method(
        &mut self,
        self_: Resource<OutgoingRequest>,
        method: Method,
    ) -> wasmtime::Result<Result<(), ()>> {
        <T as latest::http::types::HostOutgoingRequest>::set_method(self, self_, method.into())
    }

    fn path_with_query(
        &mut self,
        self_: Resource<OutgoingRequest>,
    ) -> wasmtime::Result<Option<String>> {
        <T as latest::http::types::HostOutgoingRequest>::path_with_query(self, self_)
    }

    fn set_path_with_query(
        &mut self,
        self_: Resource<OutgoingRequest>,
        path_with_query: Option<String>,
    ) -> wasmtime::Result<Result<(), ()>> {
        <T as latest::http::types::HostOutgoingRequest>::set_path_with_query(
            self,
            self_,
            path_with_query,
        )
    }

    fn scheme(&mut self, self_: Resource<OutgoingRequest>) -> wasmtime::Result<Option<Scheme>> {
        <T as latest::http::types::HostOutgoingRequest>::scheme(self, self_)
            .map(|s| s.map(|s| s.into()))
    }

    fn set_scheme(
        &mut self,
        self_: Resource<OutgoingRequest>,
        scheme: Option<Scheme>,
    ) -> wasmtime::Result<Result<(), ()>> {
        <T as latest::http::types::HostOutgoingRequest>::set_scheme(
            self,
            self_,
            scheme.map(|s| s.into()),
        )
    }

    fn authority(&mut self, self_: Resource<OutgoingRequest>) -> wasmtime::Result<Option<String>> {
        <T as latest::http::types::HostOutgoingRequest>::authority(self, self_)
    }

    fn set_authority(
        &mut self,
        self_: Resource<OutgoingRequest>,
        authority: Option<String>,
    ) -> wasmtime::Result<Result<(), ()>> {
        <T as latest::http::types::HostOutgoingRequest>::set_authority(self, self_, authority)
    }

    fn headers(&mut self, self_: Resource<OutgoingRequest>) -> wasmtime::Result<Resource<Headers>> {
        <T as latest::http::types::HostOutgoingRequest>::headers(self, self_)
    }

    fn body(
        &mut self,
        self_: Resource<OutgoingRequest>,
    ) -> wasmtime::Result<Result<Resource<OutgoingBody>, ()>> {
        <T as latest::http::types::HostOutgoingRequest>::body(self, self_)
    }

    fn drop(&mut self, rep: Resource<OutgoingRequest>) -> wasmtime::Result<()> {
        <T as latest::http::types::HostOutgoingRequest>::drop(self, rep)
    }
}

impl<T> wasi::http::types::HostOutgoingResponse for T
where
    T: WasiHttpView,
{
    fn new(&mut self, headers: Resource<Headers>) -> wasmtime::Result<Resource<OutgoingResponse>> {
        let headers = <T as latest::http::types::HostFields>::clone(self, headers)?;
        <T as latest::http::types::HostOutgoingResponse>::new(self, headers)
    }

    fn status_code(&mut self, self_: Resource<OutgoingResponse>) -> wasmtime::Result<StatusCode> {
        <T as latest::http::types::HostOutgoingResponse>::status_code(self, self_)
    }

    fn set_status_code(
        &mut self,
        self_: Resource<OutgoingResponse>,
        status_code: StatusCode,
    ) -> wasmtime::Result<Result<(), ()>> {
        <T as latest::http::types::HostOutgoingResponse>::set_status_code(self, self_, status_code)
    }

    fn headers(
        &mut self,
        self_: Resource<OutgoingResponse>,
    ) -> wasmtime::Result<Resource<Headers>> {
        <T as latest::http::types::HostOutgoingResponse>::headers(self, self_)
    }

    fn body(
        &mut self,
        self_: Resource<OutgoingResponse>,
    ) -> wasmtime::Result<Result<Resource<OutgoingBody>, ()>> {
        <T as latest::http::types::HostOutgoingResponse>::body(self, self_)
    }

    fn drop(&mut self, rep: Resource<OutgoingResponse>) -> wasmtime::Result<()> {
        <T as latest::http::types::HostOutgoingResponse>::drop(self, rep)
    }
}

impl<T> wasi::http::types::HostOutgoingBody for T
where
    T: WasiHttpView,
{
    fn write(
        &mut self,
        self_: Resource<OutgoingBody>,
    ) -> wasmtime::Result<Result<Resource<OutputStream>, ()>> {
        <T as latest::http::types::HostOutgoingBody>::write(self, self_)
    }

    fn finish(
        &mut self,
        this: Resource<OutgoingBody>,
        trailers: Option<Resource<Trailers>>,
    ) -> wasmtime::Result<Result<(), HttpErrorCode>> {
        <T as latest::http::types::HostOutgoingBody>::finish(self, this, trailers)
            .map(|r| r.map_err(|e| e.into()))
    }

    fn drop(&mut self, rep: Resource<OutgoingBody>) -> wasmtime::Result<()> {
        <T as latest::http::types::HostOutgoingBody>::drop(self, rep)
    }
}

impl<T> wasi::http::types::HostResponseOutparam for T
where
    T: WasiHttpView,
{
    fn set(
        &mut self,
        param: Resource<ResponseOutparam>,
        response: Result<Resource<OutgoingResponse>, HttpErrorCode>,
    ) -> wasmtime::Result<()> {
        <T as latest::http::types::HostResponseOutparam>::set(
            self,
            param,
            response.map_err(|e| e.into()),
        )
    }

    fn drop(&mut self, rep: Resource<ResponseOutparam>) -> wasmtime::Result<()> {
        <T as latest::http::types::HostResponseOutparam>::drop(self, rep)
    }
}

impl<T> wasi::http::types::HostFutureTrailers for T
where
    T: WasiHttpView,
{
    fn subscribe(
        &mut self,
        self_: Resource<FutureTrailers>,
    ) -> wasmtime::Result<Resource<Pollable>> {
        <T as latest::http::types::HostFutureTrailers>::subscribe(self, self_)
    }

    fn get(
        &mut self,
        self_: Resource<FutureTrailers>,
    ) -> wasmtime::Result<Option<Result<Option<Resource<Trailers>>, HttpErrorCode>>> {
        match <T as latest::http::types::HostFutureTrailers>::get(self, self_)? {
            Some(Ok(Ok(trailers))) => Ok(Some(Ok(trailers))),
            Some(Ok(Err(e))) => Ok(Some(Err(e.into()))),
            Some(Err(())) => Err(anyhow::anyhow!("trailers have already been retrieved")),
            None => Ok(None),
        }
    }

    fn drop(&mut self, rep: Resource<FutureTrailers>) -> wasmtime::Result<()> {
        <T as latest::http::types::HostFutureTrailers>::drop(self, rep)
    }
}

impl<T> wasi::http::types::HostFutureIncomingResponse for T
where
    T: WasiHttpView,
{
    fn get(
        &mut self,
        self_: Resource<FutureIncomingResponse>,
    ) -> wasmtime::Result<Option<Result<Result<Resource<IncomingResponse>, HttpErrorCode>, ()>>>
    {
        match <T as latest::http::types::HostFutureIncomingResponse>::get(self, self_)? {
            None => Ok(None),
            Some(Ok(Ok(response))) => Ok(Some(Ok(Ok(response)))),
            Some(Ok(Err(e))) => Ok(Some(Ok(Err(e.into())))),
            Some(Err(())) => Ok(Some(Err(()))),
        }
    }

    fn subscribe(
        &mut self,
        self_: Resource<FutureIncomingResponse>,
    ) -> wasmtime::Result<Resource<Pollable>> {
        <T as latest::http::types::HostFutureIncomingResponse>::subscribe(self, self_)
    }

    fn drop(&mut self, rep: Resource<FutureIncomingResponse>) -> wasmtime::Result<()> {
        <T as latest::http::types::HostFutureIncomingResponse>::drop(self, rep)
    }
}

impl<T> wasi::http::outgoing_handler::Host for T
where
    T: WasiHttpView,
{
    fn handle(
        &mut self,
        request: Resource<OutgoingRequest>,
        options: Option<Resource<RequestOptions>>,
    ) -> wasmtime::Result<Result<Resource<FutureIncomingResponse>, HttpErrorCode>> {
        match <T as latest::http::outgoing_handler::Host>::handle(self, request, options)? {
            Ok(resp) => Ok(Ok(resp)),
            Err(e) => Ok(Err(e.into())),
        }
    }
}

convert! {
    struct latest::clocks::wall_clock::Datetime [<=>] Datetime {
        seconds,
        nanoseconds,
    }

    enum latest::http::types::Method [<=>] Method {
        Get,
        Head,
        Post,
        Put,
        Delete,
        Connect,
        Options,
        Trace,
        Patch,
        Other(e),
    }

    enum latest::http::types::Scheme [<=>] Scheme {
        Http,
        Https,
        Other(e),
    }

    enum latest::http::types::HeaderError => HeaderError {
        InvalidSyntax,
        Forbidden,
        Immutable,
    }

    struct latest::http::types::DnsErrorPayload [<=>] DnsErrorPayload {
        rcode,
        info_code,
    }

    struct latest::http::types::TlsAlertReceivedPayload [<=>] TlsAlertReceivedPayload {
        alert_id,
        alert_message,
    }

    struct latest::http::types::FieldSizePayload [<=>] FieldSizePayload {
        field_name,
        field_size,
    }
}

impl From<latest::http::types::ErrorCode> for HttpErrorCode {
    fn from(e: latest::http::types::ErrorCode) -> Self {
        match e {
            latest::http::types::ErrorCode::DnsTimeout => HttpErrorCode::DnsTimeout,
            latest::http::types::ErrorCode::DnsError(e) => HttpErrorCode::DnsError(e.into()),
            latest::http::types::ErrorCode::DestinationNotFound => {
                HttpErrorCode::DestinationNotFound
            }
            latest::http::types::ErrorCode::DestinationUnavailable => {
                HttpErrorCode::DestinationUnavailable
            }
            latest::http::types::ErrorCode::DestinationIpProhibited => {
                HttpErrorCode::DestinationIpProhibited
            }
            latest::http::types::ErrorCode::DestinationIpUnroutable => {
                HttpErrorCode::DestinationIpUnroutable
            }
            latest::http::types::ErrorCode::ConnectionRefused => HttpErrorCode::ConnectionRefused,
            latest::http::types::ErrorCode::ConnectionTerminated => {
                HttpErrorCode::ConnectionTerminated
            }
            latest::http::types::ErrorCode::ConnectionTimeout => HttpErrorCode::ConnectionTimeout,
            latest::http::types::ErrorCode::ConnectionReadTimeout => {
                HttpErrorCode::ConnectionReadTimeout
            }
            latest::http::types::ErrorCode::ConnectionWriteTimeout => {
                HttpErrorCode::ConnectionWriteTimeout
            }
            latest::http::types::ErrorCode::ConnectionLimitReached => {
                HttpErrorCode::ConnectionLimitReached
            }
            latest::http::types::ErrorCode::TlsProtocolError => HttpErrorCode::TlsProtocolError,
            latest::http::types::ErrorCode::TlsCertificateError => {
                HttpErrorCode::TlsCertificateError
            }
            latest::http::types::ErrorCode::TlsAlertReceived(e) => {
                HttpErrorCode::TlsAlertReceived(e.into())
            }
            latest::http::types::ErrorCode::HttpRequestDenied => HttpErrorCode::HttpRequestDenied,
            latest::http::types::ErrorCode::HttpRequestLengthRequired => {
                HttpErrorCode::HttpRequestLengthRequired
            }
            latest::http::types::ErrorCode::HttpRequestBodySize(e) => {
                HttpErrorCode::HttpRequestBodySize(e)
            }
            latest::http::types::ErrorCode::HttpRequestMethodInvalid => {
                HttpErrorCode::HttpRequestMethodInvalid
            }
            latest::http::types::ErrorCode::HttpRequestUriInvalid => {
                HttpErrorCode::HttpRequestUriInvalid
            }
            latest::http::types::ErrorCode::HttpRequestUriTooLong => {
                HttpErrorCode::HttpRequestUriTooLong
            }
            latest::http::types::ErrorCode::HttpRequestHeaderSectionSize(e) => {
                HttpErrorCode::HttpRequestHeaderSectionSize(e)
            }
            latest::http::types::ErrorCode::HttpRequestHeaderSize(e) => {
                HttpErrorCode::HttpRequestHeaderSize(e.map(|e| e.into()))
            }
            latest::http::types::ErrorCode::HttpRequestTrailerSectionSize(e) => {
                HttpErrorCode::HttpRequestTrailerSectionSize(e)
            }
            latest::http::types::ErrorCode::HttpRequestTrailerSize(e) => {
                HttpErrorCode::HttpRequestTrailerSize(e.into())
            }
            latest::http::types::ErrorCode::HttpResponseIncomplete => {
                HttpErrorCode::HttpResponseIncomplete
            }
            latest::http::types::ErrorCode::HttpResponseHeaderSectionSize(e) => {
                HttpErrorCode::HttpResponseHeaderSectionSize(e)
            }
            latest::http::types::ErrorCode::HttpResponseHeaderSize(e) => {
                HttpErrorCode::HttpResponseHeaderSize(e.into())
            }
            latest::http::types::ErrorCode::HttpResponseBodySize(e) => {
                HttpErrorCode::HttpResponseBodySize(e)
            }
            latest::http::types::ErrorCode::HttpResponseTrailerSectionSize(e) => {
                HttpErrorCode::HttpResponseTrailerSectionSize(e)
            }
            latest::http::types::ErrorCode::HttpResponseTrailerSize(e) => {
                HttpErrorCode::HttpResponseTrailerSize(e.into())
            }
            latest::http::types::ErrorCode::HttpResponseTransferCoding(e) => {
                HttpErrorCode::HttpResponseTransferCoding(e)
            }
            latest::http::types::ErrorCode::HttpResponseContentCoding(e) => {
                HttpErrorCode::HttpResponseContentCoding(e)
            }
            latest::http::types::ErrorCode::HttpResponseTimeout => {
                HttpErrorCode::HttpResponseTimeout
            }
            latest::http::types::ErrorCode::HttpUpgradeFailed => HttpErrorCode::HttpUpgradeFailed,
            latest::http::types::ErrorCode::HttpProtocolError => HttpErrorCode::HttpProtocolError,
            latest::http::types::ErrorCode::LoopDetected => HttpErrorCode::LoopDetected,
            latest::http::types::ErrorCode::ConfigurationError => HttpErrorCode::ConfigurationError,
            latest::http::types::ErrorCode::InternalError(e) => HttpErrorCode::InternalError(e),
        }
    }
}

impl From<HttpErrorCode> for latest::http::types::ErrorCode {
    fn from(e: HttpErrorCode) -> Self {
        match e {
            HttpErrorCode::DnsTimeout => latest::http::types::ErrorCode::DnsTimeout,
            HttpErrorCode::DnsError(e) => latest::http::types::ErrorCode::DnsError(e.into()),
            HttpErrorCode::DestinationNotFound => {
                latest::http::types::ErrorCode::DestinationNotFound
            }
            HttpErrorCode::DestinationUnavailable => {
                latest::http::types::ErrorCode::DestinationUnavailable
            }
            HttpErrorCode::DestinationIpProhibited => {
                latest::http::types::ErrorCode::DestinationIpProhibited
            }
            HttpErrorCode::DestinationIpUnroutable => {
                latest::http::types::ErrorCode::DestinationIpUnroutable
            }
            HttpErrorCode::ConnectionRefused => latest::http::types::ErrorCode::ConnectionRefused,
            HttpErrorCode::ConnectionTerminated => {
                latest::http::types::ErrorCode::ConnectionTerminated
            }
            HttpErrorCode::ConnectionTimeout => latest::http::types::ErrorCode::ConnectionTimeout,
            HttpErrorCode::ConnectionReadTimeout => {
                latest::http::types::ErrorCode::ConnectionReadTimeout
            }
            HttpErrorCode::ConnectionWriteTimeout => {
                latest::http::types::ErrorCode::ConnectionWriteTimeout
            }
            HttpErrorCode::ConnectionLimitReached => {
                latest::http::types::ErrorCode::ConnectionLimitReached
            }
            HttpErrorCode::TlsProtocolError => latest::http::types::ErrorCode::TlsProtocolError,
            HttpErrorCode::TlsCertificateError => {
                latest::http::types::ErrorCode::TlsCertificateError
            }
            HttpErrorCode::TlsAlertReceived(e) => {
                latest::http::types::ErrorCode::TlsAlertReceived(e.into())
            }
            HttpErrorCode::HttpRequestDenied => latest::http::types::ErrorCode::HttpRequestDenied,
            HttpErrorCode::HttpRequestLengthRequired => {
                latest::http::types::ErrorCode::HttpRequestLengthRequired
            }
            HttpErrorCode::HttpRequestBodySize(e) => {
                latest::http::types::ErrorCode::HttpRequestBodySize(e)
            }
            HttpErrorCode::HttpRequestMethodInvalid => {
                latest::http::types::ErrorCode::HttpRequestMethodInvalid
            }
            HttpErrorCode::HttpRequestUriInvalid => {
                latest::http::types::ErrorCode::HttpRequestUriInvalid
            }
            HttpErrorCode::HttpRequestUriTooLong => {
                latest::http::types::ErrorCode::HttpRequestUriTooLong
            }
            HttpErrorCode::HttpRequestHeaderSectionSize(e) => {
                latest::http::types::ErrorCode::HttpRequestHeaderSectionSize(e)
            }
            HttpErrorCode::HttpRequestHeaderSize(e) => {
                latest::http::types::ErrorCode::HttpRequestHeaderSize(e.map(|e| e.into()))
            }
            HttpErrorCode::HttpRequestTrailerSectionSize(e) => {
                latest::http::types::ErrorCode::HttpRequestTrailerSectionSize(e)
            }
            HttpErrorCode::HttpRequestTrailerSize(e) => {
                latest::http::types::ErrorCode::HttpRequestTrailerSize(e.into())
            }
            HttpErrorCode::HttpResponseIncomplete => {
                latest::http::types::ErrorCode::HttpResponseIncomplete
            }
            HttpErrorCode::HttpResponseHeaderSectionSize(e) => {
                latest::http::types::ErrorCode::HttpResponseHeaderSectionSize(e)
            }
            HttpErrorCode::HttpResponseHeaderSize(e) => {
                latest::http::types::ErrorCode::HttpResponseHeaderSize(e.into())
            }
            HttpErrorCode::HttpResponseBodySize(e) => {
                latest::http::types::ErrorCode::HttpResponseBodySize(e)
            }
            HttpErrorCode::HttpResponseTrailerSectionSize(e) => {
                latest::http::types::ErrorCode::HttpResponseTrailerSectionSize(e)
            }
            HttpErrorCode::HttpResponseTrailerSize(e) => {
                latest::http::types::ErrorCode::HttpResponseTrailerSize(e.into())
            }
            HttpErrorCode::HttpResponseTransferCoding(e) => {
                latest::http::types::ErrorCode::HttpResponseTransferCoding(e)
            }
            HttpErrorCode::HttpResponseContentCoding(e) => {
                latest::http::types::ErrorCode::HttpResponseContentCoding(e)
            }
            HttpErrorCode::HttpResponseTimeout => {
                latest::http::types::ErrorCode::HttpResponseTimeout
            }
            HttpErrorCode::HttpUpgradeFailed => latest::http::types::ErrorCode::HttpUpgradeFailed,
            HttpErrorCode::HttpProtocolError => latest::http::types::ErrorCode::HttpProtocolError,
            HttpErrorCode::LoopDetected => latest::http::types::ErrorCode::LoopDetected,
            HttpErrorCode::ConfigurationError => latest::http::types::ErrorCode::ConfigurationError,
            HttpErrorCode::InternalError(e) => latest::http::types::ErrorCode::InternalError(e),
        }
    }
}
