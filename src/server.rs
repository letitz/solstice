use std::io;
use std::net::ToSocketAddrs;

use mio::tcp::TcpStream;

use message::Message;

#[derive(Debug)]
enum ServerState {
    NotLoggedIn,
    LoggingIn,
    LoggedIn,
}

#[derive(Debug)]
pub struct ServerConnection {
    stream: TcpStream,
    state: ServerState,
}

impl ServerConnection {
    pub fn new(hostname: &str, port: u16) -> io::Result<Self> {
        for sock_addr in try!((hostname, port).to_socket_addrs()) {
            if let Ok(stream) = TcpStream::connect(&sock_addr) {
                return Ok(ServerConnection {
                    stream: stream,
                    state: ServerState::NotLoggedIn,
                })
            }
        }
        Err(io::Error::new(io::ErrorKind::Other,
                       format!("Cannot connect to {}:{}", hostname, port)))
    }

    pub fn stream(&self) -> &TcpStream {
        &self.stream
    }

    pub fn ready_to_read(&mut self) {
        match self.state {
            _ => ()
        }
    }

    pub fn login(&mut self) {
        let msg = Message::new(1);
    }
}

