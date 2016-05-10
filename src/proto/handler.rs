use std::fmt;
use std::io;
use std::net;
use std::net::ToSocketAddrs;
use std::sync::mpsc;

use mio;
use slab;

use config;

use super::{Intent, Stream, SendPacket};
use super::server::*;

const SERVER_TOKEN:    usize = 0;
const INIT_PEER_TOKEN: usize = 1;

/*====================*
 * REQUEST - RESPONSE *
 *====================*/

#[derive(Debug)]
pub enum Request {
    ConnectToPeer(net::Ipv4Addr, u16),
    ServerRequest(ServerRequest)
}

#[derive(Debug)]
pub enum Response {
    ConnectToPeerError(net::Ipv4Addr, u16),
    ConnectToPeerSuccess(net::Ipv4Addr, u16, usize),
    ServerResponse(ServerResponse),
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

/*======================*
 * PEER RESPONSE SENDER *
 *======================*/

pub struct PeerResponseSender(mpsc::Sender<Response>, usize);

impl SendPacket for PeerResponseSender {
    type Value = u32;
    type Error = mpsc::SendError<Response>;

    fn send_packet(&mut self, value: Self::Value) -> Result<(), Self::Error> {
        Ok(())
    }
}

/*=========*
 * HANDLER *
 *=========*/

/// This struct handles all the soulseek connections, to the server and to
/// peers.
struct Handler {
    server_stream: Stream<mio::tcp::TcpStream, ServerResponseSender>,

    peer_streams:
        slab::Slab<Stream<mio::tcp::TcpStream, PeerResponseSender>, usize>,

    client_tx: mpsc::Sender<Response>,
}

impl Handler {
    fn new(client_tx: mpsc::Sender<Response>) -> io::Result<Self> {
        let host = config::SERVER_HOST;
        let port = config::SERVER_PORT;
        let server_stream = Stream::new(
            try!(Self::connect((host, port))),
            ServerResponseSender(client_tx.clone())
        );
        info!("Connected to server at {}:{}", host, port);

        Ok(Handler {
            server_stream: server_stream,

            peer_streams: slab::Slab::new_starting_at(
                INIT_PEER_TOKEN, config::MAX_PEERS
            ),

            client_tx: client_tx,
        })
    }

    fn register(&self, event_loop: &mut mio::EventLoop<Self>) -> io::Result<()>
    {
        event_loop.register(
            self.server_stream.evented(),
            mio::Token(SERVER_TOKEN),
            mio::EventSet::readable(),
            mio::PollOpt::edge() | mio::PollOpt::oneshot()
        )
    }

    fn connect<T>(addr_spec: T) -> io::Result<mio::tcp::TcpStream>
        where T: ToSocketAddrs + fmt::Debug
    {
        for sock_addr in try!(addr_spec.to_socket_addrs()) {
            if let Ok(stream) = mio::tcp::TcpStream::connect(&sock_addr) {
                return Ok(stream)
            }
        }
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Cannot connect to {:?}", addr_spec)
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
                    mio::Token(SERVER_TOKEN),
                    event_set,
                    mio::PollOpt::edge() | mio::PollOpt::oneshot()
                ).unwrap();
            }
        }
    }

    fn connect_to_peer(&mut self, ip: net::Ipv4Addr, port: u16) {
        let vacant_entry = match self.peer_streams.vacant_entry() {
            Some(vacant_entry) => vacant_entry,
            None => {
                error!(
                    "Cannot connect to peer {}:{}: too many connections open",
                    ip, port
                );
                self.client_tx.send(
                    Response::ConnectToPeerError(ip, port)
                ).unwrap();
                return
            },
        };

        info!("Connecting to peer {}:{}", ip, port);

        let tcp_stream = match Self::connect((ip, port)) {
            Ok(tcp_stream) => tcp_stream,
            Err(err) => {
                error!("Cannot connect to peer {}:{}: {}", ip, port, err);

                self.client_tx.send(
                    Response::ConnectToPeerError(ip, port)
                ).unwrap();
                return
            }
        };

        let token = vacant_entry.index();

        let peer_stream = Stream::new(
            tcp_stream, PeerResponseSender(self.client_tx.clone(), token)
        );

        vacant_entry.insert(peer_stream);

        self.client_tx.send(
            Response::ConnectToPeerSuccess(ip, port, token)
        ).unwrap();
    }
}

impl mio::Handler for Handler {
    type Timeout = ();
    type Message = Request;

    fn ready(&mut self, event_loop: &mut mio::EventLoop<Self>,
             token: mio::Token, event_set: mio::EventSet)
    {
        if token.0 == SERVER_TOKEN {
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
            Request::ConnectToPeer(ip, port) =>
                self.connect_to_peer(ip, port),

            Request::ServerRequest(server_request) => {
                let intent = self.server_stream.on_notify(&server_request);
                self.process_server_intent(intent, event_loop);
            },
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
