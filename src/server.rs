use std::io;
use std::io::{Read, Write};
use std::net::Ipv4Addr;

use mio::{EventLoop, EventSet, Handler, PollOpt, Token};
use mio::tcp::TcpStream;

use config;
use proto::{Packet, PacketStream};
use proto::server::{
    LoginRequest,
    LoginResponse,
    ServerRequest,
    ServerResponse,
};

#[derive(Debug, Clone, Copy)]
enum State {
    NotLoggedIn,
    LoggingIn,
    LoggedIn,
}

#[derive(Debug)]
pub struct ServerConnection {
    state: State,

    token_counter: usize,

    server_token: Token,
    server_stream: PacketStream<TcpStream>,
    server_interest: EventSet,
}

impl ServerConnection {
    pub fn new(server_stream: PacketStream<TcpStream>) -> Self {
        let token_counter = 0;
        ServerConnection {
            state: State::NotLoggedIn,
            token_counter: token_counter,
            server_token: Token(token_counter),
            server_stream: server_stream,
            server_interest: EventSet::writable() | EventSet::readable(),
        }
    }

    pub fn server_writable(&mut self) {
        match self.state {
            State::NotLoggedIn => {
                println!("Logging in...");
                self.state = State::LoggingIn;
                self.server_interest = EventSet::readable();
                let request = ServerRequest::LoginRequest(LoginRequest::new(
                            config::USERNAME,
                            config::PASSWORD,
                            config::VER_MAJOR,
                            config::VER_MINOR,
                            ).unwrap());
                self.server_stream.try_write(request.to_packet().unwrap());
            },

            _ => ()
        }
    }

    pub fn server_readable(&mut self) {
        match self.server_stream.try_read() {
            Ok(Some(packet)) => {
                match ServerResponse::from_packet(packet).unwrap() {
                    ServerResponse::LoginResponse(login) => {
                        self.handle_login(login);
                    },
                    ServerResponse::UnknownResponse(code, packet) => {
                        println!("Unknown packet code {}", code);
                    },
                }
            },

            Ok(None) => (),

            Err(e) => error!("Could not read packet from server: {:?}", e),
        }
    }

    pub fn register_all<T: Handler>(&self, event_loop: &mut EventLoop<T>) {
        self.server_stream.register(event_loop, self.server_token,
                                    self.server_interest, PollOpt::edge());
    }

    fn handle_login(&mut self, login: LoginResponse) -> io::Result<()> {
        match self.state {
            State::LoggingIn => {
                match login {
                    LoginResponse::LoginOk { motd, ip, password_md5_opt } => {
                        self.state = State::LoggedIn;

                        println!("Login successful!");
                        println!("MOTD: \"{}\"", motd);
                        println!("IP address: {}", ip);

                        match password_md5_opt {
                            Some(password_md5) => {
                                println!("Password MD5: \"{}\"", password_md5);
                                println!(concat!(
                                        "Connected to official server ",
                                        "as official client"));
                            },
                            None => println!(concat!(
                                    "Connected to official server ",
                                    "as unofficial client")),
                        }
                    },

                    LoginResponse::LoginFail { reason } => {
                        self.state = State::NotLoggedIn;
                        println!("Login failed!");
                        println!("Reason: {}", reason);
                    }
                }
                Ok(())
            },

            _ => unimplemented!(),
        }
    }
}

impl Handler for ServerConnection {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, event_loop: &mut EventLoop<Self>,
             token: Token, event_set: EventSet) {
        if token == self.server_token {
            if event_set.is_writable() {
                self.server_writable();
            }
            if event_set.is_readable() {
                self.server_readable();
            }
            self.server_stream.reregister(
                event_loop, token, self.server_interest,
                PollOpt::edge() | PollOpt::oneshot())
        } else {
            unreachable!("Unknown token!");
        }
    }
}
