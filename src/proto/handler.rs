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
use super::peer;

const SERVER_TOKEN:    usize = 0;
const INIT_PEER_TOKEN: usize = 1;

type ServerStream = Stream<mio::tcp::TcpStream, ServerResponseSender>;
type PeerStream   = Stream<mio::tcp::TcpStream, PeerResponseSender>;

/*====================*
 * REQUEST - RESPONSE *
 *====================*/

#[derive(Debug)]
pub enum Request {
    PeerConnect(net::Ipv4Addr, u16),
    PeerMessage(usize, peer::Message),
    ServerRequest(ServerRequest)
}

#[derive(Debug)]
pub enum Response {
    PeerConnectionClosed(usize),
    PeerConnectionError(net::Ipv4Addr, u16),
    PeerConnectionOpen(net::Ipv4Addr, u16, usize),
    PeerMessage(peer::Message),
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
    type Value = peer::Message;
    type Error = mpsc::SendError<Response>;

    fn send_packet(&mut self, value: Self::Value) -> Result<(), Self::Error> {
        self.0.send(Response::PeerMessage(value))
    }
}

/*=========*
 * HANDLER *
 *=========*/

/// This struct handles all the soulseek connections, to the server and to
/// peers.
struct Handler {
    server_stream: ServerStream,

    peer_streams: slab::Slab<PeerStream, usize>,

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

    fn connect_to_peer(
        &mut self,
        ip: net::Ipv4Addr,
        port: u16,
        event_loop: &mut mio::EventLoop<Self>)
    {
        let vacant_entry = match self.peer_streams.vacant_entry() {
            Some(vacant_entry) => vacant_entry,
            None => {
                error!(
                    "Cannot connect to peer {}:{}: too many connections open",
                    ip, port
                );
                self.client_tx.send(
                    Response::PeerConnectionError(ip, port)
                ).unwrap();
                return
            },
        };

        info!("Connecting to peer {}:{}", ip, port);

        let mut tcp_stream = match Self::connect((ip, port)) {
            Ok(tcp_stream) => tcp_stream,
            Err(err) => {
                error!("Cannot connect to peer {}:{}: {}", ip, port, err);

                self.client_tx.send(
                    Response::PeerConnectionError(ip, port)
                ).unwrap();
                return
            }
        };

        let peer_id = vacant_entry.index();

        event_loop.register(
            &mut tcp_stream,
            mio::Token(peer_id),
            mio::EventSet::readable(),
            mio::PollOpt::edge() | mio::PollOpt::oneshot()
        ).unwrap();

        let peer_stream = Stream::new(
            tcp_stream, PeerResponseSender(self.client_tx.clone(), peer_id)
        );

        vacant_entry.insert(peer_stream);

        // This is actually false, because the socket might still be connecting
        // asynchronously.
        // We will know if the connection worked or not when we get an event
        // and try to read or write.
        // There is nothing too wrong about telling the client it worked though,
        // and closing the connection as soon as the client tries to use it,
        // at which point the client will forget about the whole thing.
        self.client_tx.send(
            Response::PeerConnectionOpen(ip, port, peer_id)
        ).unwrap();
    }

    fn process_peer_intent(
        &mut self,
        intent: Intent,
        token: mio::Token,
        event_loop: &mut mio::EventLoop<Self>)
    {
        match intent {
            Intent::Done => {
                self.peer_streams.remove(token.0);
                self.client_tx.send(Response::PeerConnectionClosed(token.0))
                    .unwrap();
            },

            Intent::Continue(event_set) => {
                if let Some(peer_stream) = self.peer_streams.get_mut(token.0) {
                    event_loop.reregister(
                        peer_stream.evented(),
                        token,
                        event_set,
                        mio::PollOpt::edge() | mio::PollOpt::oneshot()
                    ).unwrap();
                }
            },
        }
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
            let intent = match self.peer_streams.get_mut(token.0) {
                Some(peer_stream) => peer_stream.on_ready(event_set),
                None => unreachable!("Unknown token is ready"),
            };
            self.process_peer_intent(intent, token, event_loop);
        }
    }

    fn notify(&mut self, event_loop: &mut mio::EventLoop<Self>,
              request: Request)
    {
        match request {
            Request::PeerConnect(ip, port) =>
                self.connect_to_peer(ip, port, event_loop),

            Request::PeerMessage(peer_id, message) => {
                let intent = match self.peer_streams.get_mut(peer_id) {
                    Some(peer_stream) => peer_stream.on_notify(&message),
                    None => {
                        error!(
                            "Cannot send peer message {:?}: unknown id {}",
                            message, peer_id
                        );
                        return
                    }
                };
                self.process_peer_intent(
                    intent, mio::Token(peer_id), event_loop
                );
            },

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
