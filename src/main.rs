mod server;
mod message;

#[macro_use] extern crate log;
extern crate mio;
extern crate byteorder;

use mio::{EventLoop, EventSet, Handler, PollOpt, Token};

use server::ServerConnection;

const SERVER_HOST : &'static str = "server.slsknet.org";
const SERVER_PORT : u16 = 2242;

const SERVER_TOKEN : Token = Token(0);

#[derive(Debug)]
struct ConnectionHandler {
    server: ServerConnection,
}

impl ConnectionHandler {
    fn new(server: ServerConnection) -> Self {
        ConnectionHandler{ server: server }
    }
}

impl Handler for ConnectionHandler {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, event_loop: &mut EventLoop<Self>,
             token: Token, event_set: EventSet) {

        match token {
            SERVER_TOKEN => self.server.ready_to_read(),

            _ => unreachable!("Unknown token"),
        }
    }
}

fn main() {
    let server = ServerConnection::new(SERVER_HOST, SERVER_PORT).unwrap();

    println!("Connected to {:?}", &server);

    let mut event_loop = EventLoop::new().unwrap();

    event_loop.register(
        server.stream(),
        SERVER_TOKEN,
        EventSet::readable(),
        PollOpt::edge()).unwrap();

    event_loop.run(&mut ConnectionHandler::new(server)).unwrap();
}
