use std::io;

use mio::{EventLoop, EventSet, Handler, PollOpt, Token};
use mio::tcp::TcpStream;

use config;
use proto::{PacketStream};
use proto::server::*;

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
                info!("Logging in...");
                self.state = State::LoggingIn;
                self.server_interest = EventSet::readable();
                let request = ServerRequest::LoginRequest(LoginRequest::new(
                            config::USERNAME,
                            config::PASSWORD,
                            config::VER_MAJOR,
                            config::VER_MINOR,
                            ).unwrap());
                self.server_stream.try_write(request.to_packet().unwrap())
                    .unwrap();
            },

            _ => ()
        }
    }

    pub fn server_readable(&mut self) {
        match self.server_stream.try_read() {
            Ok(Some(packet)) => {
                match ServerResponse::from_packet(packet) {
                    Ok(response) =>
                        self.handle_server_response(response),

                    Err(e) =>
                        error!("Error while parsing server packet: {}", e),
                }
            },

            Ok(None) => (),

            Err(e) => error!("Error while reading server packet: {:?}", e),
        }
    }

    fn handle_server_response(&mut self, response: ServerResponse) {
        match response {
            ServerResponse::LoginResponse(response) =>
                self.handle_login_response(response),

            ServerResponse::PrivilegedUsersResponse(response) =>
                self.handle_privileged_users_response(response),

            ServerResponse::RoomListResponse(response) =>
                self.handle_room_list_response(response),

            ServerResponse::UnknownResponse(code, packet) =>
                warn!("Unknown response: code {}, size {}",
                      code, packet.bytes_remaining()),

            response => warn!("Unhandled response: {:?}", response),
        }
    }

    pub fn register_all<T: Handler>(&self, event_loop: &mut EventLoop<T>)
        -> io::Result<()>
    {
        try!(self.server_stream.register(
                event_loop, self.server_token, self.server_interest,
                PollOpt::edge()));
        Ok(())
    }

    fn handle_login_response(&mut self, login: LoginResponse) {
        match self.state {
            State::LoggingIn => {
                match login {
                    LoginResponse::LoginOk { motd, ip, password_md5_opt } => {
                        self.state = State::LoggedIn;

                        info!("Login successful!");
                        info!("MOTD: \"{}\"", motd);
                        info!("External IP address: {}", ip);

                        match password_md5_opt {
                            Some(_) => {
                                info!(concat!(
                                        "Connected to official server ",
                                        "as official client"));
                            },
                            None => info!(concat!(
                                    "Connected to official server ",
                                    "as unofficial client")),
                        }
                    },

                    LoginResponse::LoginFail { reason } => {
                        self.state = State::NotLoggedIn;
                        error!("Login failed: \"{}\"", reason);
                    }
                }
            },

            _ => unimplemented!(),
        }
    }

    fn handle_room_list_response(
        &mut self, response: RoomListResponse)
    {
        info!("Received room list: {} rooms total", response.rooms.len());
    }

    fn handle_privileged_users_response(
        &mut self, response: PrivilegedUsersResponse)
    {
        info!("Received privileged users list: {} privileged users total",
              response.users.len());
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
                PollOpt::edge() | PollOpt::oneshot()).unwrap();
        } else {
            unreachable!("Unknown token!");
        }
    }
}
