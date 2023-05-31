#![allow(unused)] // temporary, until `todo!()`s are filled in

use anyhow::{anyhow, Result};
use futures::channel::oneshot;
use hyper::Body;
use spin_common::table::Table;
use spin_core::{async_trait, HostComponent};
use std::sync::Mutex;
use types2::{Method, Scheme};

wasmtime::component::bindgen!({
    path: "../../wit/preview2",
    world: "proxy",
    async: true
});

pub struct WasiCloudComponent;

impl HostComponent for WasiCloudComponent {
    type Data = WasiCloud;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        Proxy::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        Default::default()
    }
}

pub struct IncomingRequest {
    pub method: Method,
    pub path_with_query: Option<String>,
    pub scheme: Option<Scheme>,
    pub authority: Option<String>,
    pub headers: types2::Fields,
    pub body: Mutex<Option<Body>>,
}

pub struct Fields(pub Vec<(String, Vec<u8>)>);

pub struct ResponseOutparam(pub Mutex<Option<oneshot::Sender<OutboundResponse>>>);

pub struct OutboundResponse {
    pub status: u16,
    pub headers: Vec<(String, Vec<u8>)>,
    pub body: Body,
}

#[derive(Default)]
pub struct WasiCloud {
    pub incoming_requests: Table<IncomingRequest>,
    pub fields: Table<Fields>,
    pub response_outparams: Table<ResponseOutparam>,
}

// #[async_trait]
// impl wall_clock::Host for WasiCloud {
//     async fn now(&mut self) -> Result<wall_clock::Datetime> {
//         todo!()
//     }

//     async fn resolution(&mut self) -> Result<wall_clock::Datetime> {
//         todo!()
//     }
// }

// #[async_trait]
// impl monotonic_clock::Host for WasiCloud {
//     async fn now(&mut self) -> Result<monotonic_clock::Instant> {
//         todo!()
//     }

//     async fn resolution(&mut self) -> Result<monotonic_clock::Instant> {
//         todo!()
//     }

//     async fn subscribe(
//         &mut self,
//         when: monotonic_clock::Instant,
//         absolute: bool,
//     ) -> Result<monotonic_clock::Pollable> {
//         todo!()
//     }
// }

// #[async_trait]
// impl timezone::Host for WasiCloud {
//     async fn display(
//         &mut self,
//         this: timezone::Timezone,
//         when: timezone::Datetime,
//     ) -> Result<timezone::TimezoneDisplay> {
//         todo!()
//     }

//     async fn utc_offset(
//         &mut self,
//         this: timezone::Timezone,
//         when: timezone::Datetime,
//     ) -> Result<i32> {
//         todo!()
//     }

//     async fn drop_timezone(&mut self, this: timezone::Timezone) -> Result<()> {
//         todo!()
//     }
// }

#[async_trait]
impl poll2::Host for WasiCloud {
    async fn drop_pollable(&mut self, this: poll2::Pollable) -> Result<()> {
        todo!()
    }

    async fn poll_oneoff(&mut self, pollables: Vec<poll2::Pollable>) -> Result<Vec<u8>> {
        todo!()
    }
}

// #[async_trait]
// impl random::Host for WasiCloud {
//     async fn get_random_bytes(&mut self, len: u64) -> Result<Vec<u8>> {
//         todo!()
//     }

//     async fn get_random_u64(&mut self) -> Result<u64> {
//         todo!()
//     }
// }

#[async_trait]
impl streams2::Host for WasiCloud {
    async fn read(
        &mut self,
        this: streams2::InputStream,
        len: u64,
    ) -> Result<Result<(Vec<u8>, bool), streams2::StreamError>> {
        todo!()
    }

    async fn blocking_read(
        &mut self,
        this: streams2::InputStream,
        len: u64,
    ) -> Result<Result<(Vec<u8>, bool), streams2::StreamError>> {
        todo!()
    }

    async fn skip(
        &mut self,
        this: streams2::InputStream,
        len: u64,
    ) -> Result<Result<(u64, bool), streams2::StreamError>> {
        todo!()
    }

    async fn blocking_skip(
        &mut self,
        this: streams2::InputStream,
        len: u64,
    ) -> Result<Result<(u64, bool), streams2::StreamError>> {
        todo!()
    }

    async fn subscribe_to_input_stream(
        &mut self,
        this: streams2::InputStream,
    ) -> Result<streams2::Pollable> {
        todo!()
    }

    async fn drop_input_stream(&mut self, this: streams2::InputStream) -> Result<()> {
        todo!()
    }

    async fn write(
        &mut self,
        this: streams2::OutputStream,
        buf: Vec<u8>,
    ) -> Result<Result<u64, streams2::StreamError>> {
        todo!()
    }

    async fn blocking_write(
        &mut self,
        this: streams2::OutputStream,
        buf: Vec<u8>,
    ) -> Result<Result<u64, streams2::StreamError>> {
        todo!()
    }

    async fn write_zeroes(
        &mut self,
        this: streams2::OutputStream,
        len: u64,
    ) -> Result<Result<u64, streams2::StreamError>> {
        todo!()
    }

    async fn blocking_write_zeroes(
        &mut self,
        this: streams2::OutputStream,
        len: u64,
    ) -> Result<Result<u64, streams2::StreamError>> {
        todo!()
    }

    async fn splice(
        &mut self,
        this: streams2::OutputStream,
        src: streams2::InputStream,
        len: u64,
    ) -> Result<Result<(u64, bool), streams2::StreamError>> {
        todo!()
    }

    async fn blocking_splice(
        &mut self,
        this: streams2::OutputStream,
        src: streams2::InputStream,
        len: u64,
    ) -> Result<Result<(u64, bool), streams2::StreamError>> {
        todo!()
    }

    async fn forward(
        &mut self,
        this: streams2::OutputStream,
        src: streams2::InputStream,
    ) -> Result<Result<u64, streams2::StreamError>> {
        todo!()
    }

    async fn subscribe_to_output_stream(
        &mut self,
        this: streams2::OutputStream,
    ) -> Result<streams2::Pollable> {
        todo!()
    }

    async fn drop_output_stream(&mut self, this: streams2::OutputStream) -> Result<()> {
        todo!()
    }
}

// #[async_trait]
// impl stdout::Host for WasiCloud {
//     async fn get_stdout(&mut self) -> Result<stdout::OutputStream> {
//         todo!()
//     }
// }

// #[async_trait]
// impl stderr::Host for WasiCloud {
//     async fn get_stderr(&mut self) -> Result<stderr::OutputStream> {
//         todo!()
//     }
// }

// #[async_trait]
// impl stdin::Host for WasiCloud {
//     async fn get_stdin(&mut self) -> Result<stdin::InputStream> {
//         todo!()
//     }
// }

#[async_trait]
impl types2::Host for WasiCloud {
    async fn drop_fields(&mut self, fields: types2::Fields) -> Result<()> {
        todo!()
    }

    async fn new_fields(&mut self, entries: Vec<(String, Vec<u8>)>) -> Result<types2::Fields> {
        todo!()
    }

    async fn fields_get(&mut self, fields: types2::Fields, name: String) -> Result<Vec<Vec<u8>>> {
        todo!()
    }

    async fn fields_set(
        &mut self,
        fields: types2::Fields,
        name: String,
        values: Vec<Vec<u8>>,
    ) -> Result<()> {
        todo!()
    }

    async fn fields_delete(&mut self, fields: types2::Fields, name: String) -> Result<()> {
        todo!()
    }

    async fn fields_append(
        &mut self,
        fields: types2::Fields,
        name: String,
        value: Vec<u8>,
    ) -> Result<()> {
        todo!()
    }

    async fn fields_entries(&mut self, fields: types2::Fields) -> Result<Vec<(String, Vec<u8>)>> {
        todo!()
    }

    async fn fields_clone(&mut self, fields: types2::Fields) -> Result<types2::Fields> {
        todo!()
    }

    async fn finish_incoming_stream(
        &mut self,
        s: types2::IncomingStream,
    ) -> Result<Option<types2::Trailers>> {
        todo!()
    }

    async fn finish_outgoing_stream(
        &mut self,
        s: types2::OutgoingStream,
        trailers: Option<types2::Trailers>,
    ) -> Result<()> {
        todo!()
    }

    async fn drop_incoming_request(&mut self, request: types2::IncomingRequest) -> Result<()> {
        todo!()
    }

    async fn drop_outgoing_request(&mut self, request: types2::OutgoingRequest) -> Result<()> {
        todo!()
    }

    async fn incoming_request_method(&mut self, request: types2::IncomingRequest) -> Result<Method> {
        let incoming = self
            .incoming_requests
            .get(request)
            .ok_or_else(|| anyhow!("unknown request handle"))?;

        Ok(incoming.method.clone())
    }

    async fn incoming_request_path_with_query(
        &mut self,
        request: types2::IncomingRequest,
    ) -> Result<Option<String>> {
        todo!()
    }

    async fn incoming_request_scheme(
        &mut self,
        request: types2::IncomingRequest,
    ) -> Result<Option<Scheme>> {
        todo!()
    }

    async fn incoming_request_authority(
        &mut self,
        request: types2::IncomingRequest,
    ) -> Result<Option<String>> {
        todo!()
    }

    async fn incoming_request_headers(
        &mut self,
        request: types2::IncomingRequest,
    ) -> Result<types2::Headers> {
        todo!()
    }

    async fn incoming_request_consume(
        &mut self,
        request: types2::IncomingRequest,
    ) -> Result<Result<types2::IncomingStream, ()>> {
        todo!()
    }

    async fn new_outgoing_request(
        &mut self,
        method: Method,
        path_with_query: Option<String>,
        scheme: Option<Scheme>,
        authority: Option<String>,
        headers: types2::Headers,
    ) -> Result<types2::OutgoingRequest> {
        todo!()
    }

    async fn outgoing_request_write(
        &mut self,
        request: types2::OutgoingRequest,
    ) -> Result<Result<types2::OutgoingStream, ()>> {
        todo!()
    }

    async fn drop_response_outparam(&mut self, response: types2::ResponseOutparam) -> Result<()> {
        todo!()
    }

    async fn set_response_outparam(
        &mut self,
        param: types2::ResponseOutparam,
        response: Result<types2::OutgoingResponse, types2::Error>,
    ) -> Result<Result<(), ()>> {
        todo!()
    }

    async fn drop_incoming_response(&mut self, response: types2::IncomingResponse) -> Result<()> {
        todo!()
    }

    async fn drop_outgoing_response(&mut self, response: types2::OutgoingResponse) -> Result<()> {
        todo!()
    }

    async fn incoming_response_status(
        &mut self,
        response: types2::IncomingResponse,
    ) -> Result<types2::StatusCode> {
        todo!()
    }

    async fn incoming_response_headers(
        &mut self,
        response: types2::IncomingResponse,
    ) -> Result<types2::Headers> {
        todo!()
    }

    async fn incoming_response_consume(
        &mut self,
        response: types2::IncomingResponse,
    ) -> Result<Result<types2::IncomingStream, ()>> {
        todo!()
    }

    async fn new_outgoing_response(
        &mut self,
        status_code: types2::StatusCode,
        headers: types2::Headers,
    ) -> Result<types2::OutgoingResponse> {
        todo!()
    }

    async fn outgoing_response_write(
        &mut self,
        response: types2::OutgoingResponse,
    ) -> Result<Result<types2::OutgoingStream, ()>> {
        todo!()
    }

    async fn drop_future_incoming_response(
        &mut self,
        f: types2::FutureIncomingResponse,
    ) -> Result<()> {
        todo!()
    }

    async fn future_incoming_response_get(
        &mut self,
        f: types2::FutureIncomingResponse,
    ) -> Result<Option<Result<types2::IncomingResponse, types2::Error>>> {
        todo!()
    }

    async fn listen_to_future_incoming_response(
        &mut self,
        f: types2::FutureIncomingResponse,
    ) -> Result<types2::Pollable> {
        todo!()
    }
}

#[async_trait]
impl default_outgoing_http2::Host for WasiCloud {
    async fn handle(
        &mut self,
        request: default_outgoing_http2::OutgoingRequest,
        options: Option<default_outgoing_http2::RequestOptions>,
    ) -> Result<default_outgoing_http2::FutureIncomingResponse> {
        todo!()
    }
}
