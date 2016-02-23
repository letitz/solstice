mod client;
mod config;
mod proto;

extern crate byteorder;
extern crate crypto;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate mio;

use std::io;
use std::net::ToSocketAddrs;

use mio::EventLoop;
use mio::tcp::TcpStream;

use proto::PacketStream;
use client::Client;

fn connect(hostname: &str, port: u16) -> io::Result<TcpStream> {
    for sock_addr in try!((hostname, port).to_socket_addrs()) {
        if let Ok(stream) = TcpStream::connect(&sock_addr) {
            return Ok(stream)
        }
    }
    Err(io::Error::new(io::ErrorKind::Other,
                   format!("Cannot connect to {}:{}", hostname, port)))
}

fn main() {
    env_logger::init().unwrap();

    let host = config::SERVER_HOST;
    let port = config::SERVER_PORT;
    let stream = connect(host, port).unwrap();
    info!("Connected to {}:{}", host, port);

    let mut event_loop = EventLoop::new().unwrap();

    let packet_stream = PacketStream::new(stream);
    let mut server_conn = Client::new(packet_stream);
    server_conn.register_all(&mut event_loop).unwrap();

    event_loop.run(&mut server_conn).unwrap();
}
