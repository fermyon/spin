#![doc(hidden)] // internal implementation detail used in tests and spin-trigger

use super::wasi_2023_10_18::{convert, convert_result};
use anyhow::Result;
use async_trait::async_trait;
use wasmtime::component::{Linker, Resource};
use wasmtime_wasi::WasiView;
use wasmtime_wasi_http::WasiHttpView;

mod latest {
    pub use wasmtime_wasi::bindings::*;
    pub mod http {
        pub use wasmtime_wasi_http::bindings::http::*;
    }
}

mod bindings {
    use super::latest;

    wasmtime::component::bindgen!({
        path: "wit",
        interfaces: r#"
            include wasi:http/proxy@0.2.0-rc-2023-11-10;

            // NB: this is handling the historical behavior where Spin supported
            // more than "just" this snapshot of the proxy world but additionally
            // other CLI-related interfaces.
            include wasi:cli/reactor@0.2.0-rc-2023-11-10;
        "#,
        async: {
            only_imports: [
                "[method]descriptor.advise",
                "[method]descriptor.create-directory-at",
                "[method]descriptor.get-flags",
                "[method]descriptor.get-type",
                "[method]descriptor.is-same-object",
                "[method]descriptor.link-at",
                "[method]descriptor.metadata-hash",
                "[method]descriptor.metadata-hash-at",
                "[method]descriptor.open-at",
                "[method]descriptor.read",
                "[method]descriptor.read-directory",
                "[method]descriptor.readlink-at",
                "[method]descriptor.remove-directory-at",
                "[method]descriptor.rename-at",
                "[method]descriptor.set-size",
                "[method]descriptor.set-times",
                "[method]descriptor.set-times-at",
                "[method]descriptor.stat",
                "[method]descriptor.stat-at",
                "[method]descriptor.symlink-at",
                "[method]descriptor.sync",
                "[method]descriptor.sync-data",
                "[method]descriptor.unlink-file-at",
                "[method]descriptor.write",
                "[method]input-stream.read",
                "[method]input-stream.blocking-read",
                "[method]input-stream.blocking-skip",
                "[method]input-stream.skip",
                "[method]output-stream.splice",
                "[method]output-stream.blocking-splice",
                "[method]output-stream.blocking-flush",
                "[method]output-stream.blocking-write",
                "[method]output-stream.blocking-write-and-flush",
                "[method]output-stream.blocking-write-zeroes-and-flush",
                "[method]directory-entry-stream.read-directory-entry",
                "[method]pollable.block",
                "[method]pollable.ready",
                "poll",
            ]
        },
        with: {
            "wasi:io/poll/pollable": latest::io::poll::Pollable,
            "wasi:io/streams/input-stream": latest::io::streams::InputStream,
            "wasi:io/streams/output-stream": latest::io::streams::OutputStream,
            "wasi:io/error/error": latest::io::error::Error,
            "wasi:filesystem/types/directory-entry-stream": latest::filesystem::types::DirectoryEntryStream,
            "wasi:filesystem/types/descriptor": latest::filesystem::types::Descriptor,
            "wasi:cli/terminal-input/terminal-input": latest::cli::terminal_input::TerminalInput,
            "wasi:cli/terminal-output/terminal-output": latest::cli::terminal_output::TerminalOutput,
            "wasi:sockets/tcp/tcp-socket": latest::sockets::tcp::TcpSocket,
            "wasi:sockets/udp/udp-socket": latest::sockets::udp::UdpSocket,
            "wasi:sockets/udp/outgoing-datagram-stream": latest::sockets::udp::OutgoingDatagramStream,
            "wasi:sockets/udp/incoming-datagram-stream": latest::sockets::udp::IncomingDatagramStream,
            "wasi:sockets/network/network": latest::sockets::network::Network,
            "wasi:sockets/ip-name-lookup/resolve-address-stream": latest::sockets::ip_name_lookup::ResolveAddressStream,
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
        trappable_imports: true,
        skip_mut_forwarding_impls: true,
    });
}

mod wasi {
    pub use super::bindings::wasi::{
        cli0_2_0_rc_2023_11_10 as cli, clocks0_2_0_rc_2023_11_10 as clocks,
        filesystem0_2_0_rc_2023_11_10 as filesystem, http0_2_0_rc_2023_11_10 as http,
        io0_2_0_rc_2023_11_10 as io, random0_2_0_rc_2023_11_10 as random,
        sockets0_2_0_rc_2023_11_10 as sockets,
    };
}

pub mod exports {
    pub mod wasi {
        pub use super::super::bindings::exports::wasi::http0_2_0_rc_2023_11_10 as http;
    }
}

use wasi::cli::terminal_input::TerminalInput;
use wasi::cli::terminal_output::TerminalOutput;
use wasi::clocks::monotonic_clock::{Duration, Instant};
use wasi::clocks::wall_clock::Datetime;
use wasi::filesystem::types::{
    Advice, Descriptor, DescriptorFlags, DescriptorStat, DescriptorType, DirectoryEntry,
    DirectoryEntryStream, ErrorCode as FsErrorCode, Filesize, MetadataHashValue, NewTimestamp,
    OpenFlags, PathFlags,
};
use wasi::http::types::{
    DnsErrorPayload, ErrorCode as HttpErrorCode, FieldSizePayload, Fields, FutureIncomingResponse,
    FutureTrailers, HeaderError, Headers, IncomingBody, IncomingRequest, IncomingResponse, Method,
    OutgoingBody, OutgoingRequest, OutgoingResponse, RequestOptions, ResponseOutparam, Scheme,
    StatusCode, TlsAlertReceivedPayload, Trailers,
};
use wasi::io::poll::Pollable;
use wasi::io::streams::{Error as IoError, InputStream, OutputStream, StreamError};
use wasi::sockets::ip_name_lookup::{IpAddress, ResolveAddressStream};
use wasi::sockets::network::{Ipv4SocketAddress, Ipv6SocketAddress};
use wasi::sockets::tcp::{
    ErrorCode as SocketErrorCode, IpAddressFamily, IpSocketAddress, Network, ShutdownType,
    TcpSocket,
};
use wasi::sockets::udp::{
    IncomingDatagram, IncomingDatagramStream, OutgoingDatagram, OutgoingDatagramStream, UdpSocket,
};

pub fn add_to_linker<T>(linker: &mut Linker<T>) -> Result<()>
where
    T: WasiView + WasiHttpView,
{
    // interfaces from the "command" world
    fn project<T, F>(f: F) -> F
    where
        F: Fn(&mut T) -> &mut T,
    {
        f
    }
    let closure = project::<T, _>(|t| t);
    wasi::clocks::monotonic_clock::add_to_linker_get_host(linker, closure)?;
    wasi::clocks::wall_clock::add_to_linker_get_host(linker, closure)?;
    wasi::filesystem::types::add_to_linker_get_host(linker, closure)?;
    wasi::filesystem::preopens::add_to_linker_get_host(linker, closure)?;
    wasi::io::error::add_to_linker_get_host(linker, closure)?;
    wasi::io::poll::add_to_linker_get_host(linker, closure)?;
    wasi::io::streams::add_to_linker_get_host(linker, closure)?;
    wasi::random::random::add_to_linker_get_host(linker, closure)?;
    wasi::random::insecure::add_to_linker_get_host(linker, closure)?;
    wasi::random::insecure_seed::add_to_linker_get_host(linker, closure)?;
    wasi::cli::exit::add_to_linker_get_host(linker, closure)?;
    wasi::cli::environment::add_to_linker_get_host(linker, closure)?;
    wasi::cli::stdin::add_to_linker_get_host(linker, closure)?;
    wasi::cli::stdout::add_to_linker_get_host(linker, closure)?;
    wasi::cli::stderr::add_to_linker_get_host(linker, closure)?;
    wasi::cli::terminal_input::add_to_linker_get_host(linker, closure)?;
    wasi::cli::terminal_output::add_to_linker_get_host(linker, closure)?;
    wasi::cli::terminal_stdin::add_to_linker_get_host(linker, closure)?;
    wasi::cli::terminal_stdout::add_to_linker_get_host(linker, closure)?;
    wasi::cli::terminal_stderr::add_to_linker_get_host(linker, closure)?;
    wasi::sockets::tcp::add_to_linker_get_host(linker, closure)?;
    wasi::sockets::tcp_create_socket::add_to_linker_get_host(linker, closure)?;
    wasi::sockets::udp::add_to_linker_get_host(linker, closure)?;
    wasi::sockets::udp_create_socket::add_to_linker_get_host(linker, closure)?;
    wasi::sockets::instance_network::add_to_linker_get_host(linker, closure)?;
    wasi::sockets::network::add_to_linker_get_host(linker, closure)?;
    wasi::sockets::ip_name_lookup::add_to_linker_get_host(linker, closure)?;

    wasi::http::types::add_to_linker_get_host(linker, closure)?;
    wasi::http::outgoing_handler::add_to_linker_get_host(linker, closure)?;
    Ok(())
}

impl<T> wasi::clocks::monotonic_clock::Host for T
where
    T: WasiView,
{
    fn now(&mut self) -> wasmtime::Result<Instant> {
        <T as latest::clocks::monotonic_clock::Host>::now(self)
    }

    fn resolution(&mut self) -> wasmtime::Result<Instant> {
        <T as latest::clocks::monotonic_clock::Host>::resolution(self)
    }

    fn subscribe_instant(&mut self, when: Instant) -> wasmtime::Result<Resource<Pollable>> {
        <T as latest::clocks::monotonic_clock::Host>::subscribe_instant(self, when)
    }

    fn subscribe_duration(&mut self, when: Duration) -> wasmtime::Result<Resource<Pollable>> {
        <T as latest::clocks::monotonic_clock::Host>::subscribe_duration(self, when)
    }
}

impl<T> wasi::clocks::wall_clock::Host for T
where
    T: WasiView,
{
    fn now(&mut self) -> wasmtime::Result<Datetime> {
        Ok(<T as latest::clocks::wall_clock::Host>::now(self)?.into())
    }

    fn resolution(&mut self) -> wasmtime::Result<Datetime> {
        Ok(<T as latest::clocks::wall_clock::Host>::resolution(self)?.into())
    }
}

impl<T> wasi::filesystem::types::Host for T
where
    T: WasiView,
{
    fn filesystem_error_code(
        &mut self,
        err: Resource<wasi::filesystem::types::Error>,
    ) -> wasmtime::Result<Option<FsErrorCode>> {
        Ok(
            <T as latest::filesystem::types::Host>::filesystem_error_code(self, err)?
                .map(|e| e.into()),
        )
    }
}

#[async_trait]
impl<T> wasi::filesystem::types::HostDescriptor for T
where
    T: WasiView,
{
    fn read_via_stream(
        &mut self,
        self_: Resource<Descriptor>,
        offset: Filesize,
    ) -> wasmtime::Result<Result<Resource<InputStream>, FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::read_via_stream(self, self_, offset),
        )
    }

    fn write_via_stream(
        &mut self,
        self_: Resource<Descriptor>,
        offset: Filesize,
    ) -> wasmtime::Result<Result<Resource<OutputStream>, FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::write_via_stream(self, self_, offset),
        )
    }

    fn append_via_stream(
        &mut self,
        self_: Resource<Descriptor>,
    ) -> wasmtime::Result<Result<Resource<OutputStream>, FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::append_via_stream(self, self_),
        )
    }

    async fn advise(
        &mut self,
        self_: Resource<Descriptor>,
        offset: Filesize,
        length: Filesize,
        advice: Advice,
    ) -> wasmtime::Result<Result<(), FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::advise(
                self,
                self_,
                offset,
                length,
                advice.into(),
            )
            .await,
        )
    }

    async fn sync_data(
        &mut self,
        self_: Resource<Descriptor>,
    ) -> wasmtime::Result<Result<(), FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::sync_data(self, self_).await,
        )
    }

    async fn get_flags(
        &mut self,
        self_: Resource<Descriptor>,
    ) -> wasmtime::Result<Result<DescriptorFlags, FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::get_flags(self, self_).await,
        )
    }

    async fn get_type(
        &mut self,
        self_: Resource<Descriptor>,
    ) -> wasmtime::Result<Result<DescriptorType, FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::get_type(self, self_).await,
        )
    }

    async fn set_size(
        &mut self,
        self_: Resource<Descriptor>,
        size: Filesize,
    ) -> wasmtime::Result<Result<(), FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::set_size(self, self_, size).await,
        )
    }

    async fn set_times(
        &mut self,
        self_: Resource<Descriptor>,
        data_access_timestamp: NewTimestamp,
        data_modification_timestamp: NewTimestamp,
    ) -> wasmtime::Result<Result<(), FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::set_times(
                self,
                self_,
                data_access_timestamp.into(),
                data_modification_timestamp.into(),
            )
            .await,
        )
    }

    async fn read(
        &mut self,
        self_: Resource<Descriptor>,
        length: Filesize,
        offset: Filesize,
    ) -> wasmtime::Result<Result<(Vec<u8>, bool), FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::read(self, self_, length, offset)
                .await,
        )
    }

    async fn write(
        &mut self,
        self_: Resource<Descriptor>,
        buffer: Vec<u8>,
        offset: Filesize,
    ) -> wasmtime::Result<Result<Filesize, FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::write(self, self_, buffer, offset)
                .await,
        )
    }

    async fn read_directory(
        &mut self,
        self_: Resource<Descriptor>,
    ) -> wasmtime::Result<Result<Resource<DirectoryEntryStream>, FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::read_directory(self, self_).await,
        )
    }

    async fn sync(
        &mut self,
        self_: Resource<Descriptor>,
    ) -> wasmtime::Result<Result<(), FsErrorCode>> {
        convert_result(<T as latest::filesystem::types::HostDescriptor>::sync(self, self_).await)
    }

    async fn create_directory_at(
        &mut self,
        self_: Resource<Descriptor>,
        path: String,
    ) -> wasmtime::Result<Result<(), FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::create_directory_at(
                self, self_, path,
            )
            .await,
        )
    }

    async fn stat(
        &mut self,
        self_: Resource<Descriptor>,
    ) -> wasmtime::Result<Result<DescriptorStat, FsErrorCode>> {
        convert_result(<T as latest::filesystem::types::HostDescriptor>::stat(self, self_).await)
    }

    async fn stat_at(
        &mut self,
        self_: Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
    ) -> wasmtime::Result<Result<DescriptorStat, FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::stat_at(
                self,
                self_,
                path_flags.into(),
                path,
            )
            .await,
        )
    }

    async fn set_times_at(
        &mut self,
        self_: Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
        data_access_timestamp: NewTimestamp,
        data_modification_timestamp: NewTimestamp,
    ) -> wasmtime::Result<Result<(), FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::set_times_at(
                self,
                self_,
                path_flags.into(),
                path,
                data_access_timestamp.into(),
                data_modification_timestamp.into(),
            )
            .await,
        )
    }

    async fn link_at(
        &mut self,
        self_: Resource<Descriptor>,
        old_path_flags: PathFlags,
        old_path: String,
        new_descriptor: Resource<Descriptor>,
        new_path: String,
    ) -> wasmtime::Result<Result<(), FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::link_at(
                self,
                self_,
                old_path_flags.into(),
                old_path,
                new_descriptor,
                new_path,
            )
            .await,
        )
    }

    async fn open_at(
        &mut self,
        self_: Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
        open_flags: OpenFlags,
        flags: DescriptorFlags,
    ) -> wasmtime::Result<Result<Resource<Descriptor>, FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::open_at(
                self,
                self_,
                path_flags.into(),
                path,
                open_flags.into(),
                flags.into(),
            )
            .await,
        )
    }

    async fn readlink_at(
        &mut self,
        self_: Resource<Descriptor>,
        path: String,
    ) -> wasmtime::Result<Result<String, FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::readlink_at(self, self_, path).await,
        )
    }

    async fn remove_directory_at(
        &mut self,
        self_: Resource<Descriptor>,
        path: String,
    ) -> wasmtime::Result<Result<(), FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::remove_directory_at(
                self, self_, path,
            )
            .await,
        )
    }

    async fn rename_at(
        &mut self,
        self_: Resource<Descriptor>,
        old_path: String,
        new_descriptor: Resource<Descriptor>,
        new_path: String,
    ) -> wasmtime::Result<Result<(), FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::rename_at(
                self,
                self_,
                old_path,
                new_descriptor,
                new_path,
            )
            .await,
        )
    }

    async fn symlink_at(
        &mut self,
        self_: Resource<Descriptor>,
        old_path: String,
        new_path: String,
    ) -> wasmtime::Result<Result<(), FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::symlink_at(
                self, self_, old_path, new_path,
            )
            .await,
        )
    }

    async fn unlink_file_at(
        &mut self,
        self_: Resource<Descriptor>,
        path: String,
    ) -> wasmtime::Result<Result<(), FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::unlink_file_at(self, self_, path)
                .await,
        )
    }

    async fn is_same_object(
        &mut self,
        self_: Resource<Descriptor>,
        other: Resource<Descriptor>,
    ) -> wasmtime::Result<bool> {
        <T as latest::filesystem::types::HostDescriptor>::is_same_object(self, self_, other).await
    }

    async fn metadata_hash(
        &mut self,
        self_: Resource<Descriptor>,
    ) -> wasmtime::Result<Result<MetadataHashValue, FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::metadata_hash(self, self_).await,
        )
    }

    async fn metadata_hash_at(
        &mut self,
        self_: Resource<Descriptor>,
        path_flags: PathFlags,
        path: String,
    ) -> wasmtime::Result<Result<MetadataHashValue, FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDescriptor>::metadata_hash_at(
                self,
                self_,
                path_flags.into(),
                path,
            )
            .await,
        )
    }

    fn drop(&mut self, rep: Resource<Descriptor>) -> wasmtime::Result<()> {
        <T as latest::filesystem::types::HostDescriptor>::drop(self, rep)
    }
}

#[async_trait]
impl<T> wasi::filesystem::types::HostDirectoryEntryStream for T
where
    T: WasiView,
{
    async fn read_directory_entry(
        &mut self,
        self_: Resource<DirectoryEntryStream>,
    ) -> wasmtime::Result<Result<Option<DirectoryEntry>, FsErrorCode>> {
        convert_result(
            <T as latest::filesystem::types::HostDirectoryEntryStream>::read_directory_entry(
                self, self_,
            )
            .await
            .map(|e| e.map(DirectoryEntry::from)),
        )
    }

    fn drop(&mut self, rep: Resource<DirectoryEntryStream>) -> wasmtime::Result<()> {
        <T as latest::filesystem::types::HostDirectoryEntryStream>::drop(self, rep)
    }
}

impl<T> wasi::filesystem::preopens::Host for T
where
    T: WasiView,
{
    fn get_directories(&mut self) -> wasmtime::Result<Vec<(Resource<Descriptor>, String)>> {
        <T as latest::filesystem::preopens::Host>::get_directories(self)
    }
}

#[async_trait]
impl<T> wasi::io::poll::Host for T
where
    T: WasiView,
{
    async fn poll(&mut self, list: Vec<Resource<Pollable>>) -> wasmtime::Result<Vec<u32>> {
        <T as latest::io::poll::Host>::poll(self, list).await
    }
}

#[async_trait]
impl<T> wasi::io::poll::HostPollable for T
where
    T: WasiView,
{
    async fn block(&mut self, rep: Resource<Pollable>) -> wasmtime::Result<()> {
        <T as latest::io::poll::HostPollable>::block(self, rep).await
    }

    async fn ready(&mut self, rep: Resource<Pollable>) -> wasmtime::Result<bool> {
        <T as latest::io::poll::HostPollable>::ready(self, rep).await
    }

    fn drop(&mut self, rep: Resource<Pollable>) -> wasmtime::Result<()> {
        <T as latest::io::poll::HostPollable>::drop(self, rep)
    }
}

impl<T> wasi::io::error::Host for T where T: WasiView {}

impl<T> wasi::io::error::HostError for T
where
    T: WasiView,
{
    fn to_debug_string(&mut self, self_: Resource<IoError>) -> wasmtime::Result<String> {
        <T as latest::io::error::HostError>::to_debug_string(self, self_)
    }

    fn drop(&mut self, rep: Resource<IoError>) -> wasmtime::Result<()> {
        <T as latest::io::error::HostError>::drop(self, rep)
    }
}

fn convert_stream_result<T, T2>(
    view: &mut dyn WasiView,
    result: Result<T, wasmtime_wasi::StreamError>,
) -> wasmtime::Result<Result<T2, StreamError>>
where
    T2: From<T>,
{
    match result {
        Ok(e) => Ok(Ok(e.into())),
        Err(wasmtime_wasi::StreamError::Closed) => Ok(Err(StreamError::Closed)),
        Err(wasmtime_wasi::StreamError::LastOperationFailed(e)) => {
            let e = view.table().push(e)?;
            Ok(Err(StreamError::LastOperationFailed(e)))
        }
        Err(wasmtime_wasi::StreamError::Trap(e)) => Err(e),
    }
}

impl<T> wasi::io::streams::Host for T where T: WasiView {}

#[async_trait]
impl<T> wasi::io::streams::HostInputStream for T
where
    T: WasiView,
{
    async fn read(
        &mut self,
        self_: Resource<InputStream>,
        len: u64,
    ) -> wasmtime::Result<Result<Vec<u8>, StreamError>> {
        let result = <T as latest::io::streams::HostInputStream>::read(self, self_, len).await;
        convert_stream_result(self, result)
    }

    async fn blocking_read(
        &mut self,
        self_: Resource<InputStream>,
        len: u64,
    ) -> wasmtime::Result<Result<Vec<u8>, StreamError>> {
        let result =
            <T as latest::io::streams::HostInputStream>::blocking_read(self, self_, len).await;
        convert_stream_result(self, result)
    }

    async fn skip(
        &mut self,
        self_: Resource<InputStream>,
        len: u64,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        let result = <T as latest::io::streams::HostInputStream>::skip(self, self_, len).await;
        convert_stream_result(self, result)
    }

    async fn blocking_skip(
        &mut self,
        self_: Resource<InputStream>,
        len: u64,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        let result =
            <T as latest::io::streams::HostInputStream>::blocking_skip(self, self_, len).await;
        convert_stream_result(self, result)
    }

    fn subscribe(&mut self, self_: Resource<InputStream>) -> wasmtime::Result<Resource<Pollable>> {
        <T as latest::io::streams::HostInputStream>::subscribe(self, self_)
    }

    fn drop(&mut self, rep: Resource<InputStream>) -> wasmtime::Result<()> {
        <T as latest::io::streams::HostInputStream>::drop(self, rep)
    }
}

#[async_trait]
impl<T> wasi::io::streams::HostOutputStream for T
where
    T: WasiView,
{
    fn check_write(
        &mut self,
        self_: Resource<OutputStream>,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        let result = <T as latest::io::streams::HostOutputStream>::check_write(self, self_);
        convert_stream_result(self, result)
    }

    fn write(
        &mut self,
        self_: Resource<OutputStream>,
        contents: Vec<u8>,
    ) -> wasmtime::Result<Result<(), StreamError>> {
        let result = <T as latest::io::streams::HostOutputStream>::write(self, self_, contents);
        convert_stream_result(self, result)
    }

    async fn blocking_write_and_flush(
        &mut self,
        self_: Resource<OutputStream>,
        contents: Vec<u8>,
    ) -> wasmtime::Result<Result<(), StreamError>> {
        let result = <T as latest::io::streams::HostOutputStream>::blocking_write_and_flush(
            self, self_, contents,
        )
        .await;
        convert_stream_result(self, result)
    }

    fn flush(
        &mut self,
        self_: Resource<OutputStream>,
    ) -> wasmtime::Result<Result<(), StreamError>> {
        let result = <T as latest::io::streams::HostOutputStream>::flush(self, self_);
        convert_stream_result(self, result)
    }

    async fn blocking_flush(
        &mut self,
        self_: Resource<OutputStream>,
    ) -> wasmtime::Result<Result<(), StreamError>> {
        let result =
            <T as latest::io::streams::HostOutputStream>::blocking_flush(self, self_).await;
        convert_stream_result(self, result)
    }

    fn subscribe(&mut self, self_: Resource<OutputStream>) -> wasmtime::Result<Resource<Pollable>> {
        <T as latest::io::streams::HostOutputStream>::subscribe(self, self_)
    }

    fn write_zeroes(
        &mut self,
        self_: Resource<OutputStream>,
        len: u64,
    ) -> wasmtime::Result<Result<(), StreamError>> {
        let result = <T as latest::io::streams::HostOutputStream>::write_zeroes(self, self_, len);
        convert_stream_result(self, result)
    }

    async fn blocking_write_zeroes_and_flush(
        &mut self,
        self_: Resource<OutputStream>,
        len: u64,
    ) -> wasmtime::Result<Result<(), StreamError>> {
        let result = <T as latest::io::streams::HostOutputStream>::blocking_write_zeroes_and_flush(
            self, self_, len,
        )
        .await;
        convert_stream_result(self, result)
    }

    async fn splice(
        &mut self,
        self_: Resource<OutputStream>,
        src: Resource<InputStream>,
        len: u64,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        let result =
            <T as latest::io::streams::HostOutputStream>::splice(self, self_, src, len).await;
        convert_stream_result(self, result)
    }

    async fn blocking_splice(
        &mut self,
        self_: Resource<OutputStream>,
        src: Resource<InputStream>,
        len: u64,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        let result =
            <T as latest::io::streams::HostOutputStream>::blocking_splice(self, self_, src, len)
                .await;
        convert_stream_result(self, result)
    }

    fn drop(&mut self, rep: Resource<OutputStream>) -> wasmtime::Result<()> {
        <T as latest::io::streams::HostOutputStream>::drop(self, rep)
    }
}

impl<T> wasi::random::random::Host for T
where
    T: WasiView,
{
    fn get_random_bytes(&mut self, len: u64) -> wasmtime::Result<Vec<u8>> {
        <T as latest::random::random::Host>::get_random_bytes(self, len)
    }

    fn get_random_u64(&mut self) -> wasmtime::Result<u64> {
        <T as latest::random::random::Host>::get_random_u64(self)
    }
}

impl<T> wasi::random::insecure::Host for T
where
    T: WasiView,
{
    fn get_insecure_random_bytes(&mut self, len: u64) -> wasmtime::Result<Vec<u8>> {
        <T as latest::random::insecure::Host>::get_insecure_random_bytes(self, len)
    }

    fn get_insecure_random_u64(&mut self) -> wasmtime::Result<u64> {
        <T as latest::random::insecure::Host>::get_insecure_random_u64(self)
    }
}

impl<T> wasi::random::insecure_seed::Host for T
where
    T: WasiView,
{
    fn insecure_seed(&mut self) -> wasmtime::Result<(u64, u64)> {
        <T as latest::random::insecure_seed::Host>::insecure_seed(self)
    }
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

impl<T> wasi::sockets::tcp::Host for T where T: WasiView {}

impl<T> wasi::sockets::tcp::HostTcpSocket for T
where
    T: WasiView,
{
    fn start_bind(
        &mut self,
        self_: Resource<TcpSocket>,
        network: Resource<Network>,
        local_address: IpSocketAddress,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(<T as latest::sockets::tcp::HostTcpSocket>::start_bind(
            self,
            self_,
            network,
            local_address.into(),
        ))
    }

    fn finish_bind(
        &mut self,
        self_: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(<T as latest::sockets::tcp::HostTcpSocket>::finish_bind(
            self, self_,
        ))
    }

    fn start_connect(
        &mut self,
        self_: Resource<TcpSocket>,
        network: Resource<Network>,
        remote_address: IpSocketAddress,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(<T as latest::sockets::tcp::HostTcpSocket>::start_connect(
            self,
            self_,
            network,
            remote_address.into(),
        ))
    }

    fn finish_connect(
        &mut self,
        self_: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<(Resource<InputStream>, Resource<OutputStream>), SocketErrorCode>>
    {
        convert_result(<T as latest::sockets::tcp::HostTcpSocket>::finish_connect(
            self, self_,
        ))
    }

    fn start_listen(
        &mut self,
        self_: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(<T as latest::sockets::tcp::HostTcpSocket>::start_listen(
            self, self_,
        ))
    }

    fn finish_listen(
        &mut self,
        self_: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(<T as latest::sockets::tcp::HostTcpSocket>::finish_listen(
            self, self_,
        ))
    }

    fn accept(
        &mut self,
        self_: Resource<TcpSocket>,
    ) -> wasmtime::Result<
        Result<
            (
                Resource<TcpSocket>,
                Resource<InputStream>,
                Resource<OutputStream>,
            ),
            SocketErrorCode,
        >,
    > {
        convert_result(<T as latest::sockets::tcp::HostTcpSocket>::accept(
            self, self_,
        ))
    }

    fn local_address(
        &mut self,
        self_: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<IpSocketAddress, SocketErrorCode>> {
        convert_result(<T as latest::sockets::tcp::HostTcpSocket>::local_address(
            self, self_,
        ))
    }

    fn remote_address(
        &mut self,
        self_: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<IpSocketAddress, SocketErrorCode>> {
        convert_result(<T as latest::sockets::tcp::HostTcpSocket>::remote_address(
            self, self_,
        ))
    }

    fn address_family(&mut self, self_: Resource<TcpSocket>) -> wasmtime::Result<IpAddressFamily> {
        <T as latest::sockets::tcp::HostTcpSocket>::address_family(self, self_).map(|e| e.into())
    }

    fn ipv6_only(
        &mut self,
        _self_: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<bool, SocketErrorCode>> {
        anyhow::bail!("ipv6-only API no longer supported")
    }

    fn set_ipv6_only(
        &mut self,
        _self_: Resource<TcpSocket>,
        _value: bool,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        anyhow::bail!("ipv6-only API no longer supported")
    }

    fn set_listen_backlog_size(
        &mut self,
        self_: Resource<TcpSocket>,
        value: u64,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(
            <T as latest::sockets::tcp::HostTcpSocket>::set_listen_backlog_size(self, self_, value),
        )
    }

    fn is_listening(&mut self, self_: Resource<TcpSocket>) -> wasmtime::Result<bool> {
        <T as latest::sockets::tcp::HostTcpSocket>::is_listening(self, self_)
    }

    fn keep_alive_enabled(
        &mut self,
        self_: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<bool, SocketErrorCode>> {
        convert_result(<T as latest::sockets::tcp::HostTcpSocket>::keep_alive_enabled(self, self_))
    }

    fn set_keep_alive_enabled(
        &mut self,
        self_: Resource<TcpSocket>,
        value: bool,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(
            <T as latest::sockets::tcp::HostTcpSocket>::set_keep_alive_enabled(self, self_, value),
        )
    }

    fn keep_alive_idle_time(
        &mut self,
        self_: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<Duration, SocketErrorCode>> {
        convert_result(
            <T as latest::sockets::tcp::HostTcpSocket>::keep_alive_idle_time(self, self_),
        )
    }

    fn set_keep_alive_idle_time(
        &mut self,
        self_: Resource<TcpSocket>,
        value: Duration,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(
            <T as latest::sockets::tcp::HostTcpSocket>::set_keep_alive_idle_time(
                self, self_, value,
            ),
        )
    }

    fn keep_alive_interval(
        &mut self,
        self_: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<Duration, SocketErrorCode>> {
        convert_result(<T as latest::sockets::tcp::HostTcpSocket>::keep_alive_interval(self, self_))
    }

    fn set_keep_alive_interval(
        &mut self,
        self_: Resource<TcpSocket>,
        value: Duration,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(
            <T as latest::sockets::tcp::HostTcpSocket>::set_keep_alive_interval(self, self_, value),
        )
    }

    fn keep_alive_count(
        &mut self,
        self_: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<u32, SocketErrorCode>> {
        convert_result(<T as latest::sockets::tcp::HostTcpSocket>::keep_alive_count(self, self_))
    }

    fn set_keep_alive_count(
        &mut self,
        self_: Resource<TcpSocket>,
        value: u32,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(
            <T as latest::sockets::tcp::HostTcpSocket>::set_keep_alive_count(self, self_, value),
        )
    }

    fn hop_limit(
        &mut self,
        self_: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<u8, SocketErrorCode>> {
        convert_result(<T as latest::sockets::tcp::HostTcpSocket>::hop_limit(
            self, self_,
        ))
    }

    fn set_hop_limit(
        &mut self,
        self_: Resource<TcpSocket>,
        value: u8,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(<T as latest::sockets::tcp::HostTcpSocket>::set_hop_limit(
            self, self_, value,
        ))
    }

    fn receive_buffer_size(
        &mut self,
        self_: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<u64, SocketErrorCode>> {
        convert_result(<T as latest::sockets::tcp::HostTcpSocket>::receive_buffer_size(self, self_))
    }

    fn set_receive_buffer_size(
        &mut self,
        self_: Resource<TcpSocket>,
        value: u64,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(
            <T as latest::sockets::tcp::HostTcpSocket>::set_receive_buffer_size(self, self_, value),
        )
    }

    fn send_buffer_size(
        &mut self,
        self_: Resource<TcpSocket>,
    ) -> wasmtime::Result<Result<u64, SocketErrorCode>> {
        convert_result(<T as latest::sockets::tcp::HostTcpSocket>::send_buffer_size(self, self_))
    }

    fn set_send_buffer_size(
        &mut self,
        self_: Resource<TcpSocket>,
        value: u64,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(
            <T as latest::sockets::tcp::HostTcpSocket>::set_send_buffer_size(self, self_, value),
        )
    }

    fn subscribe(&mut self, self_: Resource<TcpSocket>) -> wasmtime::Result<Resource<Pollable>> {
        <T as latest::sockets::tcp::HostTcpSocket>::subscribe(self, self_)
    }

    fn shutdown(
        &mut self,
        self_: Resource<TcpSocket>,
        shutdown_type: ShutdownType,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(<T as latest::sockets::tcp::HostTcpSocket>::shutdown(
            self,
            self_,
            shutdown_type.into(),
        ))
    }

    fn drop(&mut self, rep: Resource<TcpSocket>) -> wasmtime::Result<()> {
        <T as latest::sockets::tcp::HostTcpSocket>::drop(self, rep)
    }
}

impl<T> wasi::sockets::tcp_create_socket::Host for T
where
    T: WasiView,
{
    fn create_tcp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> wasmtime::Result<Result<Resource<TcpSocket>, SocketErrorCode>> {
        convert_result(
            <T as latest::sockets::tcp_create_socket::Host>::create_tcp_socket(
                self,
                address_family.into(),
            ),
        )
    }
}

impl<T> wasi::sockets::udp::Host for T where T: WasiView {}

impl<T> wasi::sockets::udp::HostUdpSocket for T
where
    T: WasiView,
{
    fn start_bind(
        &mut self,
        self_: Resource<UdpSocket>,
        network: Resource<Network>,
        local_address: IpSocketAddress,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(<T as latest::sockets::udp::HostUdpSocket>::start_bind(
            self,
            self_,
            network,
            local_address.into(),
        ))
    }

    fn finish_bind(
        &mut self,
        self_: Resource<UdpSocket>,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(<T as latest::sockets::udp::HostUdpSocket>::finish_bind(
            self, self_,
        ))
    }

    fn local_address(
        &mut self,
        self_: Resource<UdpSocket>,
    ) -> wasmtime::Result<Result<IpSocketAddress, SocketErrorCode>> {
        convert_result(<T as latest::sockets::udp::HostUdpSocket>::local_address(
            self, self_,
        ))
    }

    fn remote_address(
        &mut self,
        self_: Resource<UdpSocket>,
    ) -> wasmtime::Result<Result<IpSocketAddress, SocketErrorCode>> {
        convert_result(<T as latest::sockets::udp::HostUdpSocket>::remote_address(
            self, self_,
        ))
    }

    fn address_family(&mut self, self_: Resource<UdpSocket>) -> wasmtime::Result<IpAddressFamily> {
        <T as latest::sockets::udp::HostUdpSocket>::address_family(self, self_).map(|e| e.into())
    }

    fn ipv6_only(
        &mut self,
        _self_: Resource<UdpSocket>,
    ) -> wasmtime::Result<Result<bool, SocketErrorCode>> {
        anyhow::bail!("ipv6-only API no longer supported")
    }

    fn set_ipv6_only(
        &mut self,
        _self_: Resource<UdpSocket>,
        _value: bool,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        anyhow::bail!("ipv6-only API no longer supported")
    }

    fn unicast_hop_limit(
        &mut self,
        self_: Resource<UdpSocket>,
    ) -> wasmtime::Result<Result<u8, SocketErrorCode>> {
        convert_result(<T as latest::sockets::udp::HostUdpSocket>::unicast_hop_limit(self, self_))
    }

    fn set_unicast_hop_limit(
        &mut self,
        self_: Resource<UdpSocket>,
        value: u8,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(
            <T as latest::sockets::udp::HostUdpSocket>::set_unicast_hop_limit(self, self_, value),
        )
    }

    fn receive_buffer_size(
        &mut self,
        self_: Resource<UdpSocket>,
    ) -> wasmtime::Result<Result<u64, SocketErrorCode>> {
        convert_result(<T as latest::sockets::udp::HostUdpSocket>::receive_buffer_size(self, self_))
    }

    fn set_receive_buffer_size(
        &mut self,
        self_: Resource<UdpSocket>,
        value: u64,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(
            <T as latest::sockets::udp::HostUdpSocket>::set_receive_buffer_size(self, self_, value),
        )
    }

    fn send_buffer_size(
        &mut self,
        self_: Resource<UdpSocket>,
    ) -> wasmtime::Result<Result<u64, SocketErrorCode>> {
        convert_result(<T as latest::sockets::udp::HostUdpSocket>::send_buffer_size(self, self_))
    }

    fn set_send_buffer_size(
        &mut self,
        self_: Resource<UdpSocket>,
        value: u64,
    ) -> wasmtime::Result<Result<(), SocketErrorCode>> {
        convert_result(
            <T as latest::sockets::udp::HostUdpSocket>::set_send_buffer_size(self, self_, value),
        )
    }

    fn stream(
        &mut self,
        self_: Resource<UdpSocket>,
        remote_address: Option<IpSocketAddress>,
    ) -> wasmtime::Result<
        Result<
            (
                Resource<IncomingDatagramStream>,
                Resource<OutgoingDatagramStream>,
            ),
            SocketErrorCode,
        >,
    > {
        convert_result(<T as latest::sockets::udp::HostUdpSocket>::stream(
            self,
            self_,
            remote_address.map(|a| a.into()),
        ))
    }

    fn subscribe(&mut self, self_: Resource<UdpSocket>) -> wasmtime::Result<Resource<Pollable>> {
        <T as latest::sockets::udp::HostUdpSocket>::subscribe(self, self_)
    }

    fn drop(&mut self, rep: Resource<UdpSocket>) -> wasmtime::Result<()> {
        <T as latest::sockets::udp::HostUdpSocket>::drop(self, rep)
    }
}

impl<T> wasi::sockets::udp::HostOutgoingDatagramStream for T
where
    T: WasiView,
{
    fn check_send(
        &mut self,
        self_: Resource<OutgoingDatagramStream>,
    ) -> wasmtime::Result<Result<u64, SocketErrorCode>> {
        convert_result(
            <T as latest::sockets::udp::HostOutgoingDatagramStream>::check_send(self, self_),
        )
    }

    fn send(
        &mut self,
        self_: Resource<OutgoingDatagramStream>,
        datagrams: Vec<OutgoingDatagram>,
    ) -> wasmtime::Result<Result<u64, SocketErrorCode>> {
        convert_result(
            <T as latest::sockets::udp::HostOutgoingDatagramStream>::send(
                self,
                self_,
                datagrams.into_iter().map(|d| d.into()).collect(),
            ),
        )
    }

    fn subscribe(
        &mut self,
        self_: Resource<OutgoingDatagramStream>,
    ) -> wasmtime::Result<Resource<Pollable>> {
        <T as latest::sockets::udp::HostOutgoingDatagramStream>::subscribe(self, self_)
    }

    fn drop(&mut self, rep: Resource<OutgoingDatagramStream>) -> wasmtime::Result<()> {
        <T as latest::sockets::udp::HostOutgoingDatagramStream>::drop(self, rep)
    }
}

impl<T> wasi::sockets::udp::HostIncomingDatagramStream for T
where
    T: WasiView,
{
    fn receive(
        &mut self,
        self_: Resource<IncomingDatagramStream>,
        max_results: u64,
    ) -> wasmtime::Result<Result<Vec<IncomingDatagram>, SocketErrorCode>> {
        convert_result(
            <T as latest::sockets::udp::HostIncomingDatagramStream>::receive(
                self,
                self_,
                max_results,
            ),
        )
        .map(|r| r.map(|r: Vec<_>| r.into_iter().map(|d| d.into()).collect()))
    }

    fn subscribe(
        &mut self,
        self_: Resource<IncomingDatagramStream>,
    ) -> wasmtime::Result<Resource<Pollable>> {
        <T as latest::sockets::udp::HostIncomingDatagramStream>::subscribe(self, self_)
    }

    fn drop(&mut self, rep: Resource<IncomingDatagramStream>) -> wasmtime::Result<()> {
        <T as latest::sockets::udp::HostIncomingDatagramStream>::drop(self, rep)
    }
}

impl<T> wasi::sockets::udp_create_socket::Host for T
where
    T: WasiView,
{
    fn create_udp_socket(
        &mut self,
        address_family: IpAddressFamily,
    ) -> wasmtime::Result<Result<Resource<UdpSocket>, SocketErrorCode>> {
        convert_result(
            <T as latest::sockets::udp_create_socket::Host>::create_udp_socket(
                self,
                address_family.into(),
            ),
        )
    }
}

impl<T> wasi::sockets::instance_network::Host for T
where
    T: WasiView,
{
    fn instance_network(&mut self) -> wasmtime::Result<Resource<Network>> {
        <T as latest::sockets::instance_network::Host>::instance_network(self)
    }
}

impl<T> wasi::sockets::network::Host for T where T: WasiView {}

impl<T> wasi::sockets::network::HostNetwork for T
where
    T: WasiView,
{
    fn drop(&mut self, rep: Resource<Network>) -> wasmtime::Result<()> {
        <T as latest::sockets::network::HostNetwork>::drop(self, rep)
    }
}

impl<T> wasi::sockets::ip_name_lookup::Host for T
where
    T: WasiView,
{
    fn resolve_addresses(
        &mut self,
        network: Resource<Network>,
        name: String,
    ) -> wasmtime::Result<Result<Resource<ResolveAddressStream>, SocketErrorCode>> {
        convert_result(
            <T as latest::sockets::ip_name_lookup::Host>::resolve_addresses(self, network, name),
        )
    }
}

impl<T> wasi::sockets::ip_name_lookup::HostResolveAddressStream for T
where
    T: WasiView,
{
    fn resolve_next_address(
        &mut self,
        self_: Resource<ResolveAddressStream>,
    ) -> wasmtime::Result<Result<Option<IpAddress>, SocketErrorCode>> {
        convert_result(
            <T as latest::sockets::ip_name_lookup::HostResolveAddressStream>::resolve_next_address(
                self, self_,
            )
            .map(|e| e.map(|e| e.into())),
        )
    }

    fn subscribe(
        &mut self,
        self_: Resource<ResolveAddressStream>,
    ) -> wasmtime::Result<Resource<Pollable>> {
        <T as latest::sockets::ip_name_lookup::HostResolveAddressStream>::subscribe(self, self_)
    }

    fn drop(&mut self, rep: Resource<ResolveAddressStream>) -> wasmtime::Result<()> {
        <T as latest::sockets::ip_name_lookup::HostResolveAddressStream>::drop(self, rep)
    }
}

impl<T> wasi::http::types::Host for T
where
    T: WasiHttpView + Send,
{
    fn http_error_code(
        &mut self,
        error: Resource<IoError>,
    ) -> wasmtime::Result<Option<HttpErrorCode>> {
        <T as latest::http::types::Host>::http_error_code(self, error).map(|e| e.map(|e| e.into()))
    }
}

impl<T> wasi::http::types::HostRequestOptions for T
where
    T: WasiHttpView + Send,
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
    T: WasiHttpView + Send,
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
    T: WasiHttpView + Send,
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
    T: WasiHttpView + Send,
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
    T: WasiHttpView + Send,
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
    T: WasiHttpView + Send,
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
    T: WasiHttpView + Send,
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
    T: WasiHttpView + Send,
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
        match <T as latest::http::types::HostOutgoingBody>::finish(self, this, trailers) {
            Ok(()) => Ok(Ok(())),
            Err(e) => Ok(Err(e.downcast()?.into())),
        }
    }

    fn drop(&mut self, rep: Resource<OutgoingBody>) -> wasmtime::Result<()> {
        <T as latest::http::types::HostOutgoingBody>::drop(self, rep)
    }
}

impl<T> wasi::http::types::HostResponseOutparam for T
where
    T: WasiHttpView + Send,
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
    T: WasiHttpView + Send,
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
    T: WasiHttpView + Send,
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
    T: WasiHttpView + Send,
{
    fn handle(
        &mut self,
        request: Resource<OutgoingRequest>,
        options: Option<Resource<RequestOptions>>,
    ) -> wasmtime::Result<Result<Resource<FutureIncomingResponse>, HttpErrorCode>> {
        match <T as latest::http::outgoing_handler::Host>::handle(self, request, options) {
            Ok(resp) => Ok(Ok(resp)),
            Err(e) => Ok(Err(e.downcast()?.into())),
        }
    }
}

convert! {
    struct latest::clocks::wall_clock::Datetime [<=>] Datetime {
        seconds,
        nanoseconds,
    }

    enum latest::filesystem::types::ErrorCode => FsErrorCode {
        Access,
        WouldBlock,
        Already,
        BadDescriptor,
        Busy,
        Deadlock,
        Quota,
        Exist,
        FileTooLarge,
        IllegalByteSequence,
        InProgress,
        Interrupted,
        Invalid,
        Io,
        IsDirectory,
        Loop,
        TooManyLinks,
        MessageSize,
        NameTooLong,
        NoDevice,
        NoEntry,
        NoLock,
        InsufficientMemory,
        InsufficientSpace,
        NotDirectory,
        NotEmpty,
        NotRecoverable,
        Unsupported,
        NoTty,
        NoSuchDevice,
        Overflow,
        NotPermitted,
        Pipe,
        ReadOnly,
        InvalidSeek,
        TextFileBusy,
        CrossDevice,
    }

    enum Advice => latest::filesystem::types::Advice {
        Normal,
        Sequential,
        Random,
        WillNeed,
        DontNeed,
        NoReuse,
    }

    flags DescriptorFlags [<=>] latest::filesystem::types::DescriptorFlags {
        READ,
        WRITE,
        FILE_INTEGRITY_SYNC,
        DATA_INTEGRITY_SYNC,
        REQUESTED_WRITE_SYNC,
        MUTATE_DIRECTORY,
    }

    enum DescriptorType [<=>] latest::filesystem::types::DescriptorType {
        Unknown,
        BlockDevice,
        CharacterDevice,
        Directory,
        Fifo,
        SymbolicLink,
        RegularFile,
        Socket,
    }

    enum NewTimestamp => latest::filesystem::types::NewTimestamp {
        NoChange,
        Now,
        Timestamp(e),
    }

    flags PathFlags => latest::filesystem::types::PathFlags {
        SYMLINK_FOLLOW,
    }

    flags OpenFlags => latest::filesystem::types::OpenFlags {
        CREATE,
        DIRECTORY,
        EXCLUSIVE,
        TRUNCATE,
    }

    struct latest::filesystem::types::MetadataHashValue => MetadataHashValue {
        lower,
        upper,
    }

    struct latest::filesystem::types::DirectoryEntry => DirectoryEntry {
        type_,
        name,
    }


    enum latest::sockets::network::ErrorCode => SocketErrorCode {
        Unknown,
        AccessDenied,
        NotSupported,
        InvalidArgument,
        OutOfMemory,
        Timeout,
        ConcurrencyConflict,
        NotInProgress,
        WouldBlock,
        InvalidState,
        NewSocketLimit,
        AddressNotBindable,
        AddressInUse,
        RemoteUnreachable,
        ConnectionRefused,
        ConnectionReset,
        ConnectionAborted,
        DatagramTooLarge,
        NameUnresolvable,
        TemporaryResolverFailure,
        PermanentResolverFailure,
    }

    enum latest::sockets::network::IpAddress [<=>] IpAddress {
        Ipv4(e),
        Ipv6(e),
    }

    enum latest::sockets::network::IpSocketAddress [<=>] IpSocketAddress {
        Ipv4(e),
        Ipv6(e),
    }

    struct latest::sockets::network::Ipv4SocketAddress [<=>] Ipv4SocketAddress {
        port,
        address,
    }

    struct latest::sockets::network::Ipv6SocketAddress [<=>] Ipv6SocketAddress {
        port,
        flow_info,
        scope_id,
        address,
    }

    enum latest::sockets::network::IpAddressFamily [<=>] IpAddressFamily {
        Ipv4,
        Ipv6,
    }

    enum ShutdownType => latest::sockets::tcp::ShutdownType {
        Receive,
        Send,
        Both,
    }

    struct latest::sockets::udp::IncomingDatagram => IncomingDatagram {
        data,
        remote_address,
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

impl From<latest::filesystem::types::DescriptorStat> for DescriptorStat {
    fn from(e: latest::filesystem::types::DescriptorStat) -> DescriptorStat {
        DescriptorStat {
            type_: e.type_.into(),
            link_count: e.link_count,
            size: e.size,
            data_access_timestamp: e.data_access_timestamp.map(|e| e.into()),
            data_modification_timestamp: e.data_modification_timestamp.map(|e| e.into()),
            status_change_timestamp: e.status_change_timestamp.map(|e| e.into()),
        }
    }
}

impl From<OutgoingDatagram> for latest::sockets::udp::OutgoingDatagram {
    fn from(d: OutgoingDatagram) -> Self {
        Self {
            data: d.data,
            remote_address: d.remote_address.map(|a| a.into()),
        }
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
