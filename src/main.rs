#![feature(mpsc_select)]

mod client;
mod config;
mod control;
mod handler;
mod proto;

extern crate byteorder;
extern crate core;
extern crate crypto;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate mio;
extern crate rustc_serialize;
extern crate websocket;

use std::sync::mpsc::channel;
use std::thread;

use mio::EventLoop;

use client::Client;
use control::Controller;
use handler::ConnectionHandler;

fn main() {
    env_logger::init().unwrap();

    let mut event_loop = EventLoop::new().unwrap();

    let (handler_to_client_tx, handler_to_client_rx) = channel();
    let (control_to_client_tx, control_to_client_rx) = channel();
    let (client_to_control_tx, client_to_control_rx) = channel();
    let client_to_handler_tx = event_loop.channel();

    let mut handler = ConnectionHandler::new(
        config::SERVER_HOST, config::SERVER_PORT,
        handler_to_client_tx, &mut event_loop).unwrap();

    let mut client = Client::new(
        client_to_handler_tx, handler_to_client_rx,
        client_to_control_tx, control_to_client_rx);

    let mut controller =
        Controller::new(control_to_client_tx, client_to_control_rx);

    thread::spawn(move || controller.run());
    thread::spawn(move || event_loop.run(&mut handler).unwrap());
    client.run();
}
