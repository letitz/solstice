use std::collections::VecDeque;
use std::io;
use std::net::ToSocketAddrs;
use std::sync::mpsc::Sender;

use mio::{EventLoop, EventSet, Handler, PollOpt, Token};
use mio::tcp::TcpStream;

use proto::{Packet, PacketStream, Request, Response};
use proto::server::*;

struct TokenCounter {
    counter: usize,
}

impl TokenCounter {
    fn new() -> Self {
        TokenCounter {
            counter: 0,
        }
    }

    fn next(&mut self) -> Token {
        self.counter += 1;
        Token(self.counter - 1)
    }
}

pub struct ConnectionHandler {
    token_counter: TokenCounter,

    server_token: Token,
    server_stream: PacketStream<TcpStream>,
    server_queue: VecDeque<Packet>,

    client_tx: Sender<Response>,
}

impl ConnectionHandler {
    pub fn new(
        server_host: &str,
        server_port: u16,
        client_tx: Sender<Response>,
        event_loop: &mut EventLoop<Self>)
        -> io::Result<Self>
    {
        let server_tcp_stream = try!(Self::connect(server_host, server_port));
        let server_stream = PacketStream::new(server_tcp_stream);
        info!("Connected to server at {}:{}", server_host, server_port);

        let mut token_counter = TokenCounter::new();
        let server_token = token_counter.next();

        let event_set = EventSet::readable();
        let poll_opt = PollOpt::edge() | PollOpt::oneshot();

        try!(server_stream.register(
                event_loop, server_token, event_set, poll_opt));

        Ok(ConnectionHandler {
            token_counter: token_counter,

            server_token: server_token,
            server_stream: server_stream,
            server_queue: VecDeque::new(),

            client_tx: client_tx,
        })
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

    fn read_server(&mut self) {
        loop {
            let mut packet = match self.server_stream.try_read() {
                Ok(Some(packet)) => packet,
                Ok(None)         => break,
                Err(err)         => {
                    error!("Error reading server: {}", err);
                    break
                }
            };

            debug!("Read packet with size {}", packet.bytes_remaining());

            let response = match packet.read_value() {
                Ok(resp) => {
                    debug!("Received server response: {:?}", resp);
                    Response::ServerResponse(resp)
                },
                Err(err) => {
                    error!("Error parsing server packet: {}", err);
                    break
                }
            };

            if let Err(err) = self.client_tx.send(response) {
                error!("Error sending server response to client: {}", err);
                break
            }
        }
    }

    fn write_server(&mut self) {
        loop {
            let mut packet = match self.server_queue.pop_front() {
                Some(packet) => packet,
                None => break
            };

            match self.server_stream.try_write(&mut packet) {
                Ok(Some(())) => (), // continue looping
                Ok(None)     => {
                    self.server_queue.push_front(packet);
                    break
                },
                Err(e) => {
                    error!("Error writing server stream: {}", e);
                    break
                }
            }
        }
    }

    fn notify_server(&mut self, request: ServerRequest) -> io::Result<()> {
        debug!("Sending server request: {:?}", request);
        let packet = try!(request.to_packet());
        self.server_queue.push_back(packet);
        Ok(())
    }

    /// Re-register the server socket with the event loop.
    fn reregister_server(&mut self, event_loop: &mut EventLoop<Self>) {
        let event_set = if self.server_queue.len() > 0 {
            EventSet::readable() | EventSet::writable()
        } else {
            EventSet::readable()
        };

        let poll_opt = PollOpt::edge() | PollOpt::oneshot();

        self.server_stream.reregister(
            event_loop, self.server_token, event_set, poll_opt
        ).unwrap();
    }
}

impl Handler for ConnectionHandler {
    type Timeout = ();
    type Message = Request;

    fn ready(&mut self, event_loop: &mut EventLoop<Self>,
             token: Token, event_set: EventSet)
    {
        if token == self.server_token {
            if event_set.is_writable() {
                self.write_server();
            }
            if event_set.is_readable() {
                self.read_server();
            }
            self.reregister_server(event_loop);
        } else {
            unreachable!("Unknown token!");
        }
    }

    fn notify(&mut self, event_loop: &mut EventLoop<Self>, request: Request) {
        match request {
            Request::ServerRequest(server_request) => {
                match self.notify_server(server_request) {
                    Ok(()) => (),
                    Err(e) => error!("Error processing server request: {}", e),
                }
                self.reregister_server(event_loop);
            }
        }
    }
}
