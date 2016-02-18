use std::io;
use std::io::{Read, Write};
use std::net::Ipv4Addr;

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
pub struct ServerConnection<T: Read + Write> {
    state: State,
    server_stream: PacketStream<T>,
}

impl<T: Read + Write> ServerConnection<T> {
    pub fn new(server_stream: PacketStream<T>) -> Self {
        ServerConnection {
            state: State::NotLoggedIn,
            server_stream: server_stream,
        }
    }

    pub fn server_writable(&mut self) {
        match self.state {
            State::NotLoggedIn => {
                println!("Logging in...");
                self.state = State::LoggingIn;
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
