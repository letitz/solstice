#![feature(mpsc_select)]

mod client;
mod config;
mod control;
mod handler;
mod proto;
mod room;

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
    match env_logger::init() {
        Ok(()) => (),
        Err(err) => {
            error!("Failed to initialize logger: {}", err);
            return;
        }
    };

    let mut event_loop = match EventLoop::new() {
        Ok(event_loop) => event_loop,
        Err(err) => {
            error!("Failed to create EventLoop: {}", err);
            return;
        }
    };

    let (handler_to_client_tx, handler_to_client_rx) = channel();
    let (control_to_client_tx, control_to_client_rx) = channel();
    let (client_to_control_tx, client_to_control_rx) = channel();
    let client_to_handler_tx = event_loop.channel();

    let mut handler = {
        let handler_result = ConnectionHandler::new(
            config::SERVER_HOST, config::SERVER_PORT,
            handler_to_client_tx, &mut event_loop);

        match handler_result {
            Ok(handler) => handler,
            Err(err) => {
                error!("Failed to create ConnectionHandler: {}", err);
                return;
            }
        }
    };

    let mut client = Client::new(
        client_to_handler_tx, handler_to_client_rx,
        client_to_control_tx, control_to_client_rx);

    let mut controller =
        Controller::new(control_to_client_tx, client_to_control_rx);

    thread::spawn(move || controller.run());
    thread::spawn(move || event_loop.run(&mut handler).unwrap());
    client.run();
}
