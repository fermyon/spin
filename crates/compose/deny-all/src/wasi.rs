use crate::bindings::{
    exports::wasi::{
        cli0_2_0 as cli,
        filesystem0_2_0 as filesystem,
        http0_2_0 as http,
        sockets0_2_0 as sockets,
    },
    wasi::sockets0_2_0::network,
};
use crate::format_deny_error;

impl cli::environment::Guest for crate::Component {
    fn get_environment() -> Vec<(String, String)> {
        Vec::new()
    }

    fn get_arguments() -> Vec<String> {
        Vec::new()
    }

    fn initial_cwd() -> Option<String> {
        None
    }
}

impl filesystem::preopens::Guest for crate::Component {
    fn get_directories() -> Vec<(
        filesystem::preopens::Descriptor,
        String,
    )> {
        Vec::new()
    }
}

impl http::outgoing_handler::Guest for crate::Component {
    fn handle(
        _request: http::outgoing_handler::OutgoingRequest,
        _options: Option<http::outgoing_handler::RequestOptions>,
    ) -> Result<
        http::outgoing_handler::FutureIncomingResponse,
        http::outgoing_handler::ErrorCode,
    > {
        Err(http::outgoing_handler::ErrorCode::InternalError(Some(format_deny_error("wasi:http/outgoing-handler"))))
    }
}

pub struct ResolveAddressStream;

impl sockets::ip_name_lookup::GuestResolveAddressStream for ResolveAddressStream {
    fn resolve_next_address(
        &self,
    ) -> Result<Option<sockets::ip_name_lookup::IpAddress>, sockets::ip_name_lookup::ErrorCode>
    {
        unreachable!()
    }
    fn subscribe(&self) -> sockets::ip_name_lookup::Pollable {
        unreachable!()
    }
}

impl sockets::ip_name_lookup::Guest for crate::Component {
    type ResolveAddressStream = ResolveAddressStream;

    fn resolve_addresses(
        network: &sockets::ip_name_lookup::Network,
        name: String,
    ) -> Result<
        sockets::ip_name_lookup::ResolveAddressStream,
        sockets::ip_name_lookup::ErrorCode,
    > {
        Err(sockets::ip_name_lookup::ErrorCode::AccessDenied)
    }
}

pub struct TcpSocket;

impl sockets::tcp::GuestTcpSocket for TcpSocket {
    fn start_bind(
        &self,
        network: &sockets::tcp::Network,
        local_address: sockets::tcp::IpSocketAddress,
    ) -> Result<(), sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn finish_bind(&self) -> Result<(), sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn start_connect(
        &self,
        network: &sockets::tcp::Network,
        remote_address: sockets::tcp::IpSocketAddress,
    ) -> Result<(), sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn finish_connect(
        &self,
    ) -> Result<
        (
            sockets::tcp::InputStream,
            sockets::tcp::OutputStream,
        ),
        sockets::tcp::ErrorCode,
    > {
        unreachable!()
    }
    fn start_listen(&self) -> Result<(), sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn finish_listen(&self) -> Result<(), sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn accept(
        &self,
    ) -> Result<
        (
            sockets::tcp::TcpSocket,
            sockets::tcp::InputStream,
            sockets::tcp::OutputStream,
        ),
        sockets::tcp::ErrorCode,
    > {
        unreachable!()
    }
    fn local_address(
        &self,
    ) -> Result<sockets::tcp::IpSocketAddress, sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn remote_address(
        &self,
    ) -> Result<sockets::tcp::IpSocketAddress, sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn is_listening(&self) -> bool {
        unreachable!()
    }
    fn address_family(&self) -> sockets::tcp::IpAddressFamily {
        unreachable!()
    }
    fn set_listen_backlog_size(
        &self,
        value: u64,
    ) -> Result<(), sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn keep_alive_enabled(&self) -> Result<bool, sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn set_keep_alive_enabled(
        &self,
        value: bool,
    ) -> Result<(), sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn keep_alive_idle_time(
        &self,
    ) -> Result<
        sockets::tcp::Duration,
        sockets::tcp::ErrorCode,
    > {
        unreachable!()
    }
    fn set_keep_alive_idle_time(
        &self,
        value: sockets::tcp::Duration,
    ) -> Result<(), sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn keep_alive_interval(
        &self,
    ) -> Result<
        sockets::tcp::Duration,
        sockets::tcp::ErrorCode,
    > {
        unreachable!()
    }
    fn set_keep_alive_interval(
        &self,
        value: sockets::tcp::Duration,
    ) -> Result<(), sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn keep_alive_count(&self) -> Result<u32, sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn set_keep_alive_count(
        &self,
        value: u32,
    ) -> Result<(), sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn hop_limit(&self) -> Result<u8, sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn set_hop_limit(
        &self,
        value: u8,
    ) -> Result<(), sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn receive_buffer_size(&self) -> Result<u64, sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn set_receive_buffer_size(
        &self,
        value: u64,
    ) -> Result<(), sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn send_buffer_size(&self) -> Result<u64, sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn set_send_buffer_size(
        &self,
        value: u64,
    ) -> Result<(), sockets::tcp::ErrorCode> {
        unreachable!()
    }
    fn subscribe(&self) -> sockets::tcp::Pollable {
        unreachable!()
    }
    fn shutdown(
        &self,
        shutdown_type: sockets::tcp::ShutdownType,
    ) -> Result<(), sockets::tcp::ErrorCode> {
        unreachable!()
    }
}

impl sockets::tcp::Guest for crate::Component {
    type TcpSocket = TcpSocket;
}

impl sockets::tcp_create_socket::Guest for crate::Component {
    fn create_tcp_socket(
        address_family: network::IpAddressFamily,
    ) -> Result<sockets::tcp::TcpSocket, network::ErrorCode> {
        Err(network::ErrorCode::AccessDenied)
    }
}

impl sockets::udp_create_socket::Guest for crate::Component {
    fn create_udp_socket(
        address_family: network::IpAddressFamily,
    ) -> Result<sockets::udp::UdpSocket, network::ErrorCode> {
        Err(network::ErrorCode::AccessDenied)
    }
}

pub struct UdpSocket;

impl sockets::udp::GuestUdpSocket for UdpSocket {
    fn start_bind(
        &self,
        network: &sockets::udp::Network,
        local_address: sockets::udp::IpSocketAddress,
    ) -> Result<(), sockets::udp::ErrorCode> {
        unreachable!()
    }
    fn finish_bind(&self) -> Result<(), sockets::udp::ErrorCode> {
        unreachable!()
    }
    fn stream(
        &self,
        remote_address: Option<sockets::udp::IpSocketAddress>,
    ) -> Result<
        (
            sockets::udp::IncomingDatagramStream,
            sockets::udp::OutgoingDatagramStream,
        ),
        sockets::udp::ErrorCode,
    > {
        unreachable!()
    }
    fn local_address(
        &self,
    ) -> Result<
        sockets::udp::IpSocketAddress,
        sockets::udp::ErrorCode,
    > {
        unreachable!()
    }
    fn remote_address(
        &self,
    ) -> Result<
        sockets::udp::IpSocketAddress,
        sockets::udp::ErrorCode,
    > {
        unreachable!()
    }
    fn address_family(&self) -> sockets::udp::IpAddressFamily {
        unreachable!()
    }
    fn unicast_hop_limit(&self) -> Result<u8, sockets::udp::ErrorCode> {
        unreachable!()
    }
    fn set_unicast_hop_limit(
        &self,
        value: u8,
    ) -> Result<(), sockets::udp::ErrorCode> {
        unreachable!()
    }
    fn receive_buffer_size(&self) -> Result<u64, sockets::udp::ErrorCode> {
        unreachable!()
    }
    fn set_receive_buffer_size(
        &self,
        value: u64,
    ) -> Result<(), sockets::udp::ErrorCode> {
        unreachable!()
    }
    fn send_buffer_size(&self) -> Result<u64, sockets::udp::ErrorCode> {
        unreachable!()
    }
    fn set_send_buffer_size(
        &self,
        value: u64,
    ) -> Result<(), sockets::udp::ErrorCode> {
        unreachable!()
    }
    fn subscribe(&self) -> sockets::udp::Pollable {
        unreachable!()
    }
}

pub struct IncomingDatagramStream;

impl sockets::udp::GuestIncomingDatagramStream for IncomingDatagramStream {
    fn receive(
        &self,
        max_results: u64,
    ) -> Result<Vec<sockets::udp::IncomingDatagram>, sockets::udp::ErrorCode> {
        unreachable!()
    }
    fn subscribe(&self) -> sockets::udp::Pollable {
        unreachable!()
    }
}

pub struct OutgoingDatagramStream;

impl sockets::udp::GuestOutgoingDatagramStream for OutgoingDatagramStream {
    fn check_send(&self) -> Result<u64, sockets::udp::ErrorCode> {
        unreachable!()
    }
    fn send(
        &self,
        datagrams: Vec<sockets::udp::OutgoingDatagram>,
    ) -> Result<u64, sockets::udp::ErrorCode> {
        unreachable!()
    }
    fn subscribe(&self) -> sockets::udp::Pollable {
        unreachable!()
    }
}

impl sockets::udp::Guest for crate::Component {
    type UdpSocket = UdpSocket;
    type IncomingDatagramStream = IncomingDatagramStream;
    type OutgoingDatagramStream = OutgoingDatagramStream;
}