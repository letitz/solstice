use std::sync::mpsc;
use std::str;
use std::thread;

use rustc_serialize::json;
use websocket;
use websocket::{Receiver, Sender};

use config;
use result;

use super::request::*;
use super::response::*;

type WebSocketReceiver =
    websocket::receiver::Receiver<websocket::WebSocketStream>;

type WebSocketSender =
    websocket::sender::Sender<websocket::WebSocketStream>;

type WebSocketClient =
    websocket::Client<websocket::DataFrame, WebSocketSender, WebSocketReceiver>;

pub struct Controller {
    client_tx: mpsc::Sender<Request>,
    client_rx: mpsc::Receiver<Response>,
}

impl Controller {
    pub fn new(tx: mpsc::Sender<Request>,
               rx: mpsc::Receiver<Response>)
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
            info!("Waiting for controller client");
            let client = match Self::try_get_client(&mut server) {
                Ok(client) => client,
                Err(e) => {
                    error!("Error accepting control connection: {}", e);
                    continue;
                }
            };

            // Empty client_rx of any messages that client has sent while
            // no-one was connected.
            while let Ok(_) = self.client_rx.try_recv() { /* continue */ }

            // Notify client that a controller is connected.
            self.client_tx.send(Request::ConnectNotification).unwrap();

            let (sender, receiver) = client.split();
            let (sender_tx, sender_rx) = mpsc::channel();

            // Handle incoming messages from controller in a separate thread,
            // and forward them to the client through client_tx.
            let tx = self.client_tx.clone();
            let handle = thread::spawn(move || {
                Self::receiver_loop(receiver, tx, sender_tx);
            });

            // Handle messages from client and forward them to the controller.
            Self::sender_loop(sender, &mut self.client_rx, sender_rx);

            // Sender loop has terminated, wait for receiver loop too.
            handle.join();

            // Notify client that the controller has disconnected.
            self.client_tx.send(Request::DisconnectNotification).unwrap();
        }
    }

    fn try_get_client(server: &mut websocket::Server)
        -> result::Result<WebSocketClient>
    {
        let connection = try!(server.accept());
        let request = try!(connection.read_request());
        let client = try!(request.accept().send());
        Ok(client)
    }

    fn receiver_loop(
        mut receiver: WebSocketReceiver,
        client_tx: mpsc::Sender<Request>,
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
                websocket::message::Type::Text => {
                    let payload = message.payload;
                    match Self::handle_text_message(&payload, &client_tx) {
                        Ok(()) => (),
                        Err(e) => {
                            error!("Error handling text message: {}", e);
                        }
                    }
                },

                websocket::message::Type::Close => {
                    info!("Received close request from controller.");
                    break;
                },

                code => warn!(
                    "Unhandled websocket message with code {:?}", code
                ),
            }
        }
        info!("Shutting down websocket receiver");
        receiver.shutdown().unwrap();
        // Notify sender that the websocket is closed
        sender_tx.send(());
    }

    fn handle_text_message(
        payload_bytes: &[u8],
        client_tx: &mpsc::Sender<Request>)
        -> result::Result<()>
    {
        let payload = try!(str::from_utf8(payload_bytes));
        let control_request = try!(json::decode(payload));
        debug!("Received control request: {:?}", control_request);
        try!(client_tx.send(control_request));
        Ok(())
    }

    fn sender_loop(
        mut sender: WebSocketSender,
        client_rx: &mut mpsc::Receiver<Response>,
        sender_rx: mpsc::Receiver<()>)
    {
        loop {
            select! {
                _ = sender_rx.recv() => break,

                response_result = client_rx.recv() => {
                    let response = match response_result {
                        Ok(response) => response,
                        Err(e) => {
                            error!(
                                "Error receiving from client channel: {}", e
                            );
                            break;
                        }
                    };
                    match Self::send_response(&mut sender, response) {
                        Ok(()) => (),
                        Err(e) =>
                            error!("Error sending control response: {}", e),
                    }
                }
            }
        }
        info!("Shutting down websocket sender");
        sender.shutdown_all().unwrap();
    }

    fn send_response(sender: &mut WebSocketSender, response: Response)
        -> result::Result<()>
    {
        debug!("Sending control response: {:?}", response);
        let encoded = try!(json::encode(&response));
        let message = websocket::Message::text(encoded);
        try!(sender.send_message(&message));
        Ok(())
    }
}

