use std::io;
use std::net::Ipv4Addr;

use config;

use proto::{Peer, Packet};
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
}

impl ServerConnection {
    pub fn new() -> Self {
        ServerConnection {
            state: State::NotLoggedIn,
        }
    }

    fn read_request(&mut self) -> Option<ServerRequest> {
        match self.state {
            State::NotLoggedIn => {
                println!("Logging in...");
                self.state = State::LoggingIn;
                Some(ServerRequest::LoginRequest(LoginRequest::new(
                            config::USERNAME,
                            config::PASSWORD,
                            config::VER_MAJOR,
                            config::VER_MINOR,
                            ).unwrap()))
            },

            _ => None
        }
    }

    fn write_response(&mut self, response: ServerResponse) {
        match response {
            ServerResponse::LoginResponse(login) => {
                self.handle_login(login);
            },
            ServerResponse::UnknownResponse(code, packet) => {
                println!("Unknown packet code {}", code);
            },
        }
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

impl Peer for ServerConnection {
    fn read_packet(&mut self) -> Option<Packet> {
        match self.read_request() {
            Some(request) => {
                match request.to_packet() {
                    Ok(packet) => Some(packet),
                    Err(e) => unimplemented!(),
                }
            },
            None => None
        }
    }

    fn write_packet(&mut self, mut packet: Packet) {
        match ServerResponse::from_packet(packet) {
            Ok(response) => self.write_response(response),
            Err(e) => unimplemented!(),
        }
    }
}
