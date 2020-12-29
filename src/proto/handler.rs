use std::fmt;
use std::io;
use std::net;
use std::net::ToSocketAddrs;

use crossbeam_channel;
use mio;
use slab;

use crate::config;

use super::peer;
use super::server::*;
use super::{Intent, SendPacket, Stream};

/*===========*
 * CONSTANTS *
 *===========*/

// There are only ever MAX_PEERS peer tokens, from 0 to MAX_PEERS - 1.
// This way we ensure no overlap and eliminate the need for coordination
// between client and handler that would otherwise be needed.
const SERVER_TOKEN: usize = config::MAX_PEERS;

const LISTEN_TOKEN: usize = config::MAX_PEERS + 1;

/*====================*
 * REQUEST - RESPONSE *
 *====================*/

#[derive(Debug)]
pub enum Request {
    PeerConnect(usize, net::Ipv4Addr, u16),
    PeerMessage(usize, peer::Message),
    ServerRequest(ServerRequest),
}

#[derive(Debug)]
pub enum Response {
    PeerConnectionClosed(usize),
    PeerConnectionOpen(usize),
    PeerMessage(usize, peer::Message),
    ServerResponse(ServerResponse),
}

/*========================*
 * SERVER RESPONSE SENDER *
 *========================*/

pub struct ServerResponseSender(crossbeam_channel::Sender<Response>);

impl SendPacket for ServerResponseSender {
    type Value = ServerResponse;
    type Error = crossbeam_channel::SendError<Response>;

    fn send_packet(&mut self, value: Self::Value) -> Result<(), Self::Error> {
        self.0.send(Response::ServerResponse(value))
    }

    fn notify_open(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/*======================*
 * PEER RESPONSE SENDER *
 *======================*/

pub struct PeerResponseSender {
    sender: crossbeam_channel::Sender<Response>,
    peer_id: usize,
}

impl SendPacket for PeerResponseSender {
    type Value = peer::Message;
    type Error = crossbeam_channel::SendError<Response>;

    fn send_packet(&mut self, value: Self::Value) -> Result<(), Self::Error> {
        self.sender.send(Response::PeerMessage(self.peer_id, value))
    }

    fn notify_open(&mut self) -> Result<(), Self::Error> {
        self.sender.send(Response::PeerConnectionOpen(self.peer_id))
    }
}

/*=========*
 * HANDLER *
 *=========*/

/// This struct handles all the soulseek connections, to the server and to
/// peers.
struct Handler {
    server_stream: Stream<ServerResponseSender>,

    peer_streams: slab::Slab<Stream<PeerResponseSender>, usize>,

    listener: mio::tcp::TcpListener,

    client_tx: crossbeam_channel::Sender<Response>,
}

fn listener_bind<U>(addr_spec: U) -> io::Result<mio::tcp::TcpListener>
where
    U: ToSocketAddrs + fmt::Debug,
{
    for socket_addr in addr_spec.to_socket_addrs()? {
        if let Ok(listener) = mio::tcp::TcpListener::bind(&socket_addr) {
            return Ok(listener);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::Other,
        format!("Cannot bind to {:?}", addr_spec),
    ))
}

impl Handler {
    #[allow(deprecated)]
    fn new(
        client_tx: crossbeam_channel::Sender<Response>,
        event_loop: &mut mio::deprecated::EventLoop<Self>,
    ) -> io::Result<Self> {
        let host = config::SERVER_HOST;
        let port = config::SERVER_PORT;
        let server_stream = Stream::new((host, port), ServerResponseSender(client_tx.clone()))?;

        info!("Connected to server at {}:{}", host, port);

        let listener = listener_bind((config::LISTEN_HOST, config::LISTEN_PORT))?;
        info!(
            "Listening for connections on {}:{}",
            config::LISTEN_HOST,
            config::LISTEN_PORT
        );

        event_loop.register(
            server_stream.evented(),
            mio::Token(SERVER_TOKEN),
            mio::Ready::all(),
            mio::PollOpt::edge() | mio::PollOpt::oneshot(),
        )?;

        event_loop.register(
            &listener,
            mio::Token(LISTEN_TOKEN),
            mio::Ready::all(),
            mio::PollOpt::edge() | mio::PollOpt::oneshot(),
        )?;

        Ok(Handler {
            server_stream: server_stream,

            peer_streams: slab::Slab::new(config::MAX_PEERS),

            listener: listener,

            client_tx: client_tx,
        })
    }

    #[allow(deprecated)]
    fn connect_to_peer(
        &mut self,
        peer_id: usize,
        ip: net::Ipv4Addr,
        port: u16,
        event_loop: &mut mio::deprecated::EventLoop<Self>,
    ) -> Result<(), String> {
        let vacant_entry = match self.peer_streams.entry(peer_id) {
            None => return Err("id out of range".to_string()),

            Some(slab::Entry::Occupied(_occupied_entry)) => {
                return Err("id already taken".to_string());
            }

            Some(slab::Entry::Vacant(vacant_entry)) => vacant_entry,
        };

        info!("Opening peer connection {} to {}:{}", peer_id, ip, port);

        let sender = PeerResponseSender {
            sender: self.client_tx.clone(),
            peer_id: peer_id,
        };

        let peer_stream = match Stream::new((ip, port), sender) {
            Ok(peer_stream) => peer_stream,

            Err(err) => return Err(format!("i/o error: {}", err)),
        };

        event_loop
            .register(
                peer_stream.evented(),
                mio::Token(peer_id),
                mio::Ready::all(),
                mio::PollOpt::edge() | mio::PollOpt::oneshot(),
            )
            .unwrap();

        vacant_entry.insert(peer_stream);

        Ok(())
    }

    #[allow(deprecated)]
    fn process_server_intent(
        &mut self,
        intent: Intent,
        event_loop: &mut mio::deprecated::EventLoop<Self>,
    ) {
        match intent {
            Intent::Done => {
                error!("Server connection closed");
                // TODO notify client and shut down
            }
            Intent::Continue(event_set) => {
                event_loop
                    .reregister(
                        self.server_stream.evented(),
                        mio::Token(SERVER_TOKEN),
                        event_set,
                        mio::PollOpt::edge() | mio::PollOpt::oneshot(),
                    )
                    .unwrap();
            }
        }
    }

    #[allow(deprecated)]
    fn process_peer_intent(
        &mut self,
        intent: Intent,
        token: mio::Token,
        event_loop: &mut mio::deprecated::EventLoop<Self>,
    ) {
        match intent {
            Intent::Done => {
                self.peer_streams.remove(token.0);
                self.client_tx
                    .send(Response::PeerConnectionClosed(token.0))
                    .unwrap();
            }

            Intent::Continue(event_set) => {
                if let Some(peer_stream) = self.peer_streams.get_mut(token.0) {
                    event_loop
                        .reregister(
                            peer_stream.evented(),
                            token,
                            event_set,
                            mio::PollOpt::edge() | mio::PollOpt::oneshot(),
                        )
                        .unwrap();
                }
            }
        }
    }
}

#[allow(deprecated)]
impl mio::deprecated::Handler for Handler {
    type Timeout = ();
    type Message = Request;

    fn ready(
        &mut self,
        event_loop: &mut mio::deprecated::EventLoop<Self>,
        token: mio::Token,
        event_set: mio::Ready,
    ) {
        match token {
            mio::Token(LISTEN_TOKEN) => {
                if event_set.is_readable() {
                    // A peer wants to connect to us.
                    match self.listener.accept() {
                        Ok((_sock, addr)) => {
                            // TODO add it to peer streams
                            info!("Peer connection accepted from {}", addr);
                        }

                        Err(err) => {
                            error!("Cannot accept peer connection: {}", err);
                        }
                    }
                }
                event_loop
                    .reregister(
                        &self.listener,
                        token,
                        mio::Ready::all(),
                        mio::PollOpt::edge() | mio::PollOpt::oneshot(),
                    )
                    .unwrap();
            }

            mio::Token(SERVER_TOKEN) => {
                let intent = self.server_stream.on_ready(event_set);
                self.process_server_intent(intent, event_loop);
            }

            mio::Token(peer_id) => {
                let intent = match self.peer_streams.get_mut(peer_id) {
                    Some(peer_stream) => peer_stream.on_ready(event_set),

                    None => unreachable!("Unknown peer {} is ready", peer_id),
                };
                self.process_peer_intent(intent, token, event_loop);
            }
        }
    }

    fn notify(&mut self, event_loop: &mut mio::deprecated::EventLoop<Self>, request: Request) {
        match request {
            Request::PeerConnect(peer_id, ip, port) => {
                if let Err(err) = self.connect_to_peer(peer_id, ip, port, event_loop) {
                    error!(
                        "Cannot open peer connection {} to {}:{}: {}",
                        peer_id, ip, port, err
                    );
                    self.client_tx
                        .send(Response::PeerConnectionClosed(peer_id))
                        .unwrap();
                }
            }

            Request::PeerMessage(peer_id, message) => {
                let intent = match self.peer_streams.get_mut(peer_id) {
                    Some(peer_stream) => peer_stream.on_notify(&message),
                    None => {
                        error!(
                            "Cannot send peer message {:?}: unknown id {}",
                            message, peer_id
                        );
                        return;
                    }
                };
                self.process_peer_intent(intent, mio::Token(peer_id), event_loop);
            }

            Request::ServerRequest(server_request) => {
                let intent = self.server_stream.on_notify(&server_request);
                self.process_server_intent(intent, event_loop);
            }
        }
    }
}

#[allow(deprecated)]
pub type Sender = mio::deprecated::Sender<Request>;

pub struct Agent {
    #[allow(deprecated)]
    event_loop: mio::deprecated::EventLoop<Handler>,
    handler: Handler,
}

impl Agent {
    pub fn new(client_tx: crossbeam_channel::Sender<Response>) -> io::Result<Self> {
        // Create the event loop.
        #[allow(deprecated)]
        let mut event_loop = mio::deprecated::EventLoop::new()?;
        // Create the handler for the event loop and register the handler's
        // sockets with the event loop.
        let handler = Handler::new(client_tx, &mut event_loop)?;

        Ok(Agent {
            event_loop: event_loop,
            handler: handler,
        })
    }

    pub fn channel(&self) -> Sender {
        #[allow(deprecated)]
        self.event_loop.channel()
    }

    pub fn run(&mut self) -> io::Result<()> {
        #[allow(deprecated)]
        self.event_loop.run(&mut self.handler)
    }
}
