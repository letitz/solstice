use std::collections::VecDeque;
use std::io;
use std::net::ToSocketAddrs;
use std::sync::mpsc;

use mio;

use config;

use super::{Packet, PacketStream, Request, Response};
use super::server::*;

/// This struct provides a simple way to generate different tokens.
struct TokenCounter {
    counter: usize,
}

impl TokenCounter {
    fn new() -> Self {
        TokenCounter {
            counter: 0,
        }
    }

    fn next(&mut self) -> mio::Token {
        self.counter += 1;
        mio::Token(self.counter - 1)
    }
}

/// This struct handles all the soulseek connections, to the server and to
/// peers.
struct Handler {
    token_counter: TokenCounter,

    server_token: mio::Token,
    server_stream: PacketStream<mio::tcp::TcpStream>,
    server_queue: VecDeque<Packet>,

    client_tx: mpsc::Sender<Response>,
}

impl Handler {
    fn new(client_tx: mpsc::Sender<Response>) -> io::Result<Self> {
        let host = config::SERVER_HOST;
        let port = config::SERVER_PORT;
        let server_stream = PacketStream::new(
            try!(Self::connect(host, port))
        );
        info!("Connected to server at {}:{}", host, port);

        let mut token_counter = TokenCounter::new();
        let server_token = token_counter.next();

        Ok(Handler {
            token_counter: token_counter,

            server_token: server_token,
            server_stream: server_stream,
            server_queue: VecDeque::new(),

            client_tx: client_tx,
        })
    }

    fn register(&self, event_loop: &mut mio::EventLoop<Self>) -> io::Result<()>
    {
        self.server_stream.register(
            event_loop,
            self.server_token,
            mio::EventSet::readable(),
            mio::PollOpt::edge() | mio::PollOpt::oneshot()
        )
    }

    fn connect(hostname: &str, port: u16) -> io::Result<mio::tcp::TcpStream> {
        for sock_addr in try!((hostname, port).to_socket_addrs()) {
            if let Ok(stream) = mio::tcp::TcpStream::connect(&sock_addr) {
                return Ok(stream)
            }
        }
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Cannot connect to {}:{}", hostname, port)
        ))
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
    fn reregister_server(&mut self, event_loop: &mut mio::EventLoop<Self>) {
        let event_set = if self.server_queue.len() > 0 {
            mio::EventSet::readable() | mio::EventSet::writable()
        } else {
            mio::EventSet::readable()
        };

        self.server_stream.reregister(
            event_loop,
            self.server_token,
            event_set,
            mio::PollOpt::edge() | mio::PollOpt::oneshot()
        ).unwrap();
    }
}

impl mio::Handler for Handler {
    type Timeout = ();
    type Message = Request;

    fn ready(&mut self, event_loop: &mut mio::EventLoop<Self>,
             token: mio::Token, event_set: mio::EventSet)
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

    fn notify(
        &mut self, event_loop: &mut mio::EventLoop<Self>, request: Request)
    {
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

pub type Sender = mio::Sender<Request>;

pub struct Agent {
    event_loop: mio::EventLoop<Handler>,
    handler:    Handler,
}

impl Agent {
    pub fn new(client_tx: mpsc::Sender<Response>) -> io::Result<Self> {
        // Create the event loop.
        let mut event_loop = try!(mio::EventLoop::new());
        // Create the handler for the event loop.
        let handler = try!(Handler::new(client_tx));
        // Register the handler's sockets with the event loop.
        try!(handler.register(&mut event_loop));

        Ok(Agent {
            event_loop: event_loop,
            handler:    handler,
        })
    }

    pub fn channel(&self) -> Sender {
        self.event_loop.channel()
    }

    pub fn run(&mut self) -> io::Result<()> {
        self.event_loop.run(&mut self.handler)
    }
}
