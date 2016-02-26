use std::io;
use std::io::{Read, Write};
use std::sync::mpsc;
use std::thread;

use rustc_serialize::json;
use mio::tcp::TcpStream;
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
        loop {
            let client = Self::get_client(host, port);
            info!("Controller client connected");
            let (mut sender, mut receiver) = client.split();
            let tx = self.client_tx.clone();
            thread::spawn(move || {
                Self::receiver_loop(receiver, tx);
            });
            Self::sender_loop(sender, &mut self.client_rx);
        }
    }

    fn get_client(host: &str, port: u16) -> WebSocketClient
    {
        let mut server = websocket::Server::bind((host, port)).unwrap();
        info!("Controller bound to {}:{}", host, port);
        loop {
            match Self::try_get_client(&mut server) {
                Ok(client) => return client,
                Err(e) => error!("Error accepting control connection: {}", e),
            }
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
        tx: mpsc::Sender<client::IncomingMessage>)
    {
        for message_result in receiver.incoming_messages() {
            let message: websocket::Message = match message_result {
                Ok(message) => message,
                Err(e) => {
                    warn!("Error receiving control message {}", e);
                    continue;
                },
            };
            let payload = match message.opcode {
                websocket::message::Type::Text =>
                    String::from_utf8(message.payload.into_owned()).unwrap(),

                code => {
                    warn!("Unhandled websocket message type: {:?}", code);
                    continue;
                },
            };
            match json::decode(&payload) {
                Ok(control_request) => {
                    debug!("Received control request: {:?}", control_request);
                    tx.send(client::IncomingMessage::ControlRequest(
                            control_request));
                },
                Err(e) => warn!("Error decoding control request: {}", e),
            };
        }
    }

    fn sender_loop(
        mut sender: WebSocketSender, rx: &mut  mpsc::Receiver<ControlResponse>)
    {
        for control_response in rx.iter() {
            let encoded = json::encode(&control_response).unwrap();
            let message = websocket::Message::text(encoded);
            sender.send_message(&message).unwrap();
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
