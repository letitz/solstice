// Still no 2018 way of using the log crate without `use log::*` everywhere.
#[macro_use]
extern crate log;

use std::thread;

use crossbeam_channel;
use env_logger;

mod client;
mod config;
mod context;
mod control;
mod dispatcher;
mod executor;
mod handlers;
mod login;
mod message_handler;
mod proto;
mod room;
mod user;

fn main() {
    match env_logger::init() {
        Ok(()) => (),
        Err(err) => {
            error!("Error initializing logger: {}", err);
            return;
        }
    };

    let (proto_to_client_tx, proto_to_client_rx) =
        crossbeam_channel::unbounded();

    let mut proto_agent = match proto::Agent::new(proto_to_client_tx) {
        Ok(agent) => agent,
        Err(err) => {
            error!("Error initializing protocol agent: {}", err);
            return;
        }
    };

    let client_to_proto_tx = proto_agent.channel();
    let (control_to_client_tx, control_to_client_rx) =
        crossbeam_channel::unbounded();

    let mut client = client::Client::new(
        client_to_proto_tx,
        proto_to_client_rx,
        control_to_client_rx,
    );

    thread::spawn(move || control::listen(control_to_client_tx));
    thread::spawn(move || proto_agent.run().unwrap());
    client.run();
}
