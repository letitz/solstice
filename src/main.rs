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

const SERVER_TOKEN: Token = Token(0);

#[derive(Debug)]
struct ConnectionHandler {
    server_conn: ServerConnection<TcpStream>,
}

impl ConnectionHandler {
    fn new(server_conn: ServerConnection<TcpStream>) -> Self {
        ConnectionHandler{
            server_conn: server_conn,
        }
    }
}

impl Handler for ConnectionHandler {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, event_loop: &mut EventLoop<Self>,
             token: Token, event_set: EventSet) {

        match token {
            SERVER_TOKEN => {
                if event_set.is_readable() {
                    self.server_conn.server_readable();
                }
                if event_set.is_writable() {
                    self.server_conn.server_writable();
                }
            },

            _ => unreachable!("Unknown token"),
        }
    }
}

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

    event_loop.register(
        &stream,
        SERVER_TOKEN,
        EventSet::readable() | EventSet::writable(),
        PollOpt::level()).unwrap();

    let packet_stream = PacketStream::new(stream);
    let server_conn = ServerConnection::new(packet_stream);
    let mut handler = ConnectionHandler::new(server_conn);

    event_loop.run(&mut handler).unwrap();
}
