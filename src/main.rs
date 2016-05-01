#![feature(mpsc_select)]

mod client;
mod config;
mod control;
mod proto;
mod room;
mod user;

extern crate byteorder;
extern crate core;
extern crate crypto;
extern crate encoding;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate mio;
extern crate rustc_serialize;
extern crate ws;

use std::sync::mpsc;
use std::thread;

fn main() {
    match env_logger::init() {
        Ok(()) => (),
        Err(err) => {
            error!("Error initializing logger: {}", err);
            return;
        }
    };

    let (proto_to_client_tx, proto_to_client_rx) = mpsc::channel();

    let mut proto_agent = match proto::Agent::new(proto_to_client_tx) {
        Ok(agent) => agent,
        Err(err) => {
            error!("Error initializing protocol agent: {}", err);
            return;
        }
    };

    let client_to_proto_tx = proto_agent.channel();
    let (control_to_client_tx, control_to_client_rx) = mpsc::channel();

    let mut client = client::Client::new(
        client_to_proto_tx, proto_to_client_rx, control_to_client_rx
    );

    thread::spawn(move || control::listen(control_to_client_tx));
    thread::spawn(move || proto_agent.run().unwrap());
    client.run();
}
