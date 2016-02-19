mod server;
mod proto;
mod config;

#[macro_use] extern crate log;
extern crate mio;
extern crate byteorder;
extern crate crypto;

use std::io;
use std::net::ToSocketAddrs;

use mio::{EventLoop, EventSet, Handler, PollOpt, Token};
use mio::tcp::TcpStream;

use proto::PacketStream;
use server::ServerConnection;

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
    let host = config::SERVER_HOST;
    let port = config::SERVER_PORT;
    let stream = connect(host, port).unwrap();
    println!("Connected to {}:{}", host, port);

    let mut event_loop = EventLoop::new().unwrap();

    let packet_stream = PacketStream::new(stream);
    let mut server_conn = ServerConnection::new(packet_stream);
    server_conn.register_all(&mut event_loop);

    event_loop.run(&mut server_conn).unwrap();
}
