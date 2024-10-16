use bindings::wasi::{
    io0_2_0::poll,
    sockets0_2_0::{
        instance_network,
        network::{
            ErrorCode, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress,
        },
        tcp_create_socket,
    },
};
use helper::{ensure_eq, ensure_ok};
use std::net::SocketAddr;

helper::define_component!(Component);

impl Component {
    fn main() -> Result<(), String> {
        let address = ensure_ok!(ensure_ok!(std::env::var("ADDRESS")).parse());

        let client = ensure_ok!(tcp_create_socket::create_tcp_socket(IpAddressFamily::Ipv4));

        ensure_ok!(client.start_connect(
            &instance_network::instance_network(),
            match address {
                SocketAddr::V6(address) => {
                    let ip = address.ip().segments();
                    IpSocketAddress::Ipv6(Ipv6SocketAddress {
                        address: (ip[0], ip[1], ip[2], ip[3], ip[4], ip[5], ip[6], ip[7]),
                        port: address.port(),
                        flow_info: 0,
                        scope_id: 0,
                    })
                }
                SocketAddr::V4(address) => {
                    let ip = address.ip().octets();
                    IpSocketAddress::Ipv4(Ipv4SocketAddress {
                        address: (ip[0], ip[1], ip[2], ip[3]),
                        port: address.port(),
                    })
                }
            },
        ));

        let (rx, tx) = loop {
            match client.finish_connect() {
                Err(ErrorCode::WouldBlock) => { poll::poll(&[&client.subscribe()]); },
                result => break ensure_ok!(result),
            }
        };

        let message = b"So rested he by the Tumtum tree";
        ensure_ok!(tx.blocking_write_and_flush(message));

        let mut buffer = Vec::with_capacity(message.len());
        while buffer.len() < message.len() {
            let chunk =
                ensure_ok!(rx.blocking_read((message.len() - buffer.len()).try_into().unwrap()));
            buffer.extend(chunk);
        }
        ensure_eq!(buffer.as_slice(), message);

        Ok(())
    }
}
