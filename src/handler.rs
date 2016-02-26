use std::collections::VecDeque;
use std::io;
use std::sync::mpsc::Sender;

use mio::{EventLoop, EventSet, Handler, PollOpt, Token};
use mio::tcp::TcpStream;

use client::IncomingMessage;
use proto::{Packet, PacketStream, Request};
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

    client_tx: Sender<IncomingMessage>,
}

impl ConnectionHandler {
    pub fn new(
        server_tcp_stream: TcpStream, client_tx: Sender<IncomingMessage>,
        event_loop: &mut EventLoop<Self>) -> Self
    {
        let mut token_counter = TokenCounter::new();
        let server_token = token_counter.next();

        let event_set = EventSet::readable();
        let poll_opt = PollOpt::edge() | PollOpt::oneshot();

        let server_stream = PacketStream::new(server_tcp_stream);
        server_stream.register(event_loop, server_token, event_set, poll_opt)
            .unwrap();

        ConnectionHandler {
            token_counter: token_counter,

            server_token: server_token,
            server_stream: server_stream,
            server_queue: VecDeque::new(),

            client_tx: client_tx,
        }
    }

    fn read_server(&mut self) {
        loop {
            match self.read_server_once() {
                Ok(true) => (),
                Ok(false) => break,
                Err(e) => {
                    error!("Error reading server: {}", e);
                    break;
                }
            }
        }
    }

    fn write_server(&mut self) {
        loop {
            match self.write_server_once() {
                Ok(true) => (),
                Ok(false) => break,
                Err(e) => {
                    error!("Error writing server: {}", e);
                    break;
                }
            }
        }
    }

    fn read_server_once(&mut self) -> io::Result<bool> {
        let packet = match try!(self.server_stream.try_read()) {
            Some(packet) => packet,
            None => return Ok(false),
        };

        let server_response = try!(ServerResponse::from_packet(packet));
        let message = IncomingMessage::ServerResponse(server_response);
        match self.client_tx.send(message) {
            Ok(()) => Ok(true),
            Err(e) => Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Send failed on client_tx channel: {}", e))),

        }
    }

    fn write_server_once(&mut self) -> io::Result<bool> {
        let mut packet = match self.server_queue.pop_front() {
            Some(packet) => packet,
            None => return Ok(false),
        };

        match try!(self.server_stream.try_write(&mut packet)) {
            Some(()) => Ok(true),
            None => {
                self.server_queue.push_front(packet);
                Ok(false)
            }
        }
    }

    fn notify_server(&mut self, request: ServerRequest) -> io::Result<()> {
        let packet = try!(request.to_packet());
        self.server_queue.push_back(packet);
        Ok(())
    }

    fn reregister_server(&mut self, event_loop: &mut EventLoop<Self>) {
        let event_set = if self.server_queue.len() > 0 {
            EventSet::readable() | EventSet::writable()
        } else {
            EventSet::readable()
        };
        let poll_opt = PollOpt::edge() | PollOpt::oneshot();
        self.server_stream.reregister(
            event_loop, self.server_token, event_set, poll_opt).unwrap();
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
