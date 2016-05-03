use std::io;
use std::net::ToSocketAddrs;
use std::sync::mpsc;

use mio;

use config;

use super::{Intent, Stream, SendPacket, Request, Response};
use super::server::*;

/*===============*
 * TOKEN COUNTER *
 *===============*/

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

/*========================*
 * SERVER RESPONSE SENDER *
 *========================*/

pub struct ServerResponseSender(mpsc::Sender<Response>);

impl SendPacket for ServerResponseSender {
    type Value = ServerResponse;
    type Error = mpsc::SendError<Response>;

    fn send_packet(&mut self, value: Self::Value) -> Result<(), Self::Error> {
        self.0.send(Response::ServerResponse(value))
    }
}

/*=========*
 * HANDLER *
 *=========*/

/// This struct handles all the soulseek connections, to the server and to
/// peers.
struct Handler {
    token_counter: TokenCounter,

    server_token: mio::Token,
    server_stream: Stream<mio::tcp::TcpStream, ServerResponseSender>,

    client_tx: mpsc::Sender<Response>,
}

impl Handler {
    fn new(client_tx: mpsc::Sender<Response>) -> io::Result<Self> {
        let host = config::SERVER_HOST;
        let port = config::SERVER_PORT;
        let server_stream = Stream::new(
            try!(Self::connect(host, port)),
            ServerResponseSender(client_tx.clone())
        );
        info!("Connected to server at {}:{}", host, port);

        let mut token_counter = TokenCounter::new();
        let server_token = token_counter.next();

        Ok(Handler {
            token_counter: token_counter,

            server_token: server_token,
            server_stream: server_stream,

            client_tx: client_tx,
        })
    }

    fn register(&self, event_loop: &mut mio::EventLoop<Self>) -> io::Result<()>
    {
        event_loop.register(
            self.server_stream.evented(),
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

    fn process_server_intent(
        &mut self, intent: Intent, event_loop: &mut mio::EventLoop<Self>)
    {
        match intent {
            Intent::Done => {
                error!("Server connection closed");
                // TODO notify client and shut down
            },
            Intent::Continue(event_set) => {
                event_loop.reregister(
                    self.server_stream.evented(),
                    self.server_token,
                    event_set,
                    mio::PollOpt::edge() | mio::PollOpt::oneshot()
                ).unwrap();
            }
        }
    }
}

impl mio::Handler for Handler {
    type Timeout = ();
    type Message = Request;

    fn ready(&mut self, event_loop: &mut mio::EventLoop<Self>,
             token: mio::Token, event_set: mio::EventSet)
    {
        if token == self.server_token {
            let intent = self.server_stream.on_ready(event_set);
            self.process_server_intent(intent, event_loop);
        } else {
            unreachable!("Unknown token!");
        }
    }

    fn notify(&mut self, event_loop: &mut mio::EventLoop<Self>,
              request: Request)
    {
        match request {
            Request::ServerRequest(server_request) => {
                let intent = self.server_stream.on_notify(&server_request);
                self.process_server_intent(intent, event_loop);
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
