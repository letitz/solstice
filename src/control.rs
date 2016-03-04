use std::io;
use std::sync::mpsc;
use std::str;
use std::thread;

use rustc_serialize::json;
use websocket;
use websocket::{Receiver, Sender};

use client;
use config;

type WebSocketReceiver =
    websocket::receiver::Receiver<websocket::WebSocketStream>;

type WebSocketSender =
    websocket::sender::Sender<websocket::WebSocketStream>;

type WebSocketClient =
    websocket::Client<websocket::DataFrame, WebSocketSender, WebSocketReceiver>;

enum Error {
    IOError(io::Error),
    WebSocketError(websocket::result::WebSocketError),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IOError(err)
    }
}

pub struct Controller {
    client_tx: mpsc::Sender<client::IncomingMessage>,
    client_rx: mpsc::Receiver<ControlResponse>,
}

impl Controller {
    pub fn new(tx: mpsc::Sender<client::IncomingMessage>,
               rx: mpsc::Receiver<ControlResponse>)
        -> Self
    {
        Controller {
            client_tx: tx,
            client_rx: rx,
        }
    }

    pub fn run(&mut self) {
        let host = config::CONTROL_HOST;
        let port = config::CONTROL_PORT;
        let mut server = websocket::Server::bind((host, port)).unwrap();
        info!("Controller bound to {}:{}", host, port);

        loop {
            let client = match Self::try_get_client(&mut server) {
                Ok(client) => client,
                Err(e) => {
                    error!("Error accepting control connection: {}", e);
                    continue;
                }
            };
            info!("Controller client connected");

            let (sender, receiver) = client.split();
            let (sender_tx, sender_rx) = mpsc::channel();

            let tx = self.client_tx.clone();
            let handle = thread::spawn(move || {
                Self::receiver_loop(receiver, tx, sender_tx);
            });

            Self::sender_loop(sender, &mut self.client_rx, sender_rx);

            handle.join();
        }
    }

    fn try_get_client(server: &mut websocket::Server)
        -> io::Result<WebSocketClient>
    {
        let connection = try!(server.accept());
        let request = try!(connection.read_request());
        match request.accept().send() {
            Ok(client) => Ok(client),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }

    fn receiver_loop(
        mut receiver: WebSocketReceiver,
        client_tx: mpsc::Sender<client::IncomingMessage>,
        sender_tx: mpsc::Sender<()>)
    {
        for message_result in receiver.incoming_messages() {
            let message : websocket::message::Message = match message_result {
                Ok(message) => message,
                Err(e) => {
                    warn!("Error receving websocket message: {}", e);
                    continue;
                }
            };
            match message.opcode {
                websocket::message::Type::Text =>
                    Self::handle_text_message(&message.payload, &client_tx),

                websocket::message::Type::Close => break,

                code => warn!("Unhandled websocket message with code {:?}",
                              code),
            }
        }
        info!("Shutting down websocket receiver");
        receiver.shutdown().unwrap();
        // Notify sender that the websocket is closed
        sender_tx.send(());
    }

    fn handle_text_message(
        payload_bytes: &[u8],
        client_tx: &mpsc::Sender<client::IncomingMessage>)
    {
        let payload = match str::from_utf8(payload_bytes) {
            Ok(payload) => payload,
            Err(e) => {
                warn!("Invalid UTF8 payload: {}", e);
                return;
            },
        };

        let control_request = match json::decode(payload) {
            Ok(control_request) => control_request,
            Err(e) => {
                warn!("Invalid JSON payload: {}", e);
                return;
            }
        };

        let message = client::IncomingMessage::ControlRequest(control_request);
        match client_tx.send(message) {
            Ok(()) => (),
            Err(e) => {
                warn!("Error sending control request to client: {}", e);
            }
        }
    }

    fn sender_loop(
        mut sender: WebSocketSender,
        client_rx: &mut mpsc::Receiver<ControlResponse>,
        sender_rx: mpsc::Receiver<()>)
    {
        loop {
            select! {
                _ = sender_rx.recv() => break,

                response_result = client_rx.recv() => {
                    match response_result {
                        Ok(response) =>
                            Self::send_response(&mut sender, response),
                        Err(e) => {
                            error!("Error receving from client channel: {}", e);
                            break;
                        }
                    }
                }
            }
        }
        info!("Shutting down websocket sender");
        sender.shutdown_all().unwrap();
    }

    fn send_response(sender: &mut WebSocketSender, response: ControlResponse) {
        let message = match json::encode(&response) {
            Ok(encoded) => websocket::Message::text(encoded),
            Err(e) => {
                error!("Error encoding control_response to JSON: {}", e);
                return;
            }
        };
        match sender.send_message(&message) {
            Ok(()) => (),
            Err(e) => {
                error!("Error sending message to control client: {}", e);
                return;
            }
        }
    }
}

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum ControlRequest {
    LoginRequest(LoginRequest),
}

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum ControlResponse {
    LoginResponse(LoginResponse),
}

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum LoginResponse {
    LoginOk {
        motd: String,
    },
    LoginFail {
        reason: String,
    }
}
