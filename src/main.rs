mod client;
mod config;
mod control;
mod handler;
mod proto;

extern crate byteorder;
extern crate crypto;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate mio;
extern crate rustc_serialize;
extern crate websocket;

use std::io;
use std::net::ToSocketAddrs;
use std::sync::mpsc::channel;
use std::thread;

use mio::EventLoop;
use mio::tcp::TcpStream;

use client::Client;
use control::Controller;
use handler::ConnectionHandler;

fn connect(hostname: &str, port: u16) -> io::Result<TcpStream> {
    for sock_addr in try!((hostname, port).to_socket_addrs()) {
        if let Ok(stream) = TcpStream::connect(&sock_addr) {
            return Ok(stream)
        }
    }
    Err(io::Error::new(io::ErrorKind::Other,
                   format!("Cannot connect to {}:{}", hostname, port)))
}

fn main() {
    env_logger::init().unwrap();

    let host = config::SERVER_HOST;
    let port = config::SERVER_PORT;
    let stream = connect(host, port).unwrap();
    info!("Connected to {}:{}", host, port);

    let mut event_loop = EventLoop::new().unwrap();

    let (client_tx, client_rx) = channel();
    let (client_to_control_tx, client_to_control_rx) = channel();
    let client_to_handler_tx = event_loop.channel();

    let mut handler =
        ConnectionHandler::new(stream, client_tx.clone(), &mut event_loop);

    let mut client = Client::new(
        client_rx, client_to_handler_tx, client_to_control_tx);
    thread::spawn(move || client.run());

    let mut controller =
        Controller::new(client_tx, client_to_control_rx);
    thread::spawn(move || {
        controller.run();
    });

    event_loop.run(&mut handler).unwrap();
}
