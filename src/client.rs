use std::sync::mpsc::Receiver;

use mio::Sender;

use config;
use proto::server::*;

#[derive(Debug)]
pub enum Request {
    ServerRequest(ServerRequest),
}

#[derive(Debug)]
pub enum Response {
    ServerResponse(ServerResponse),
}

#[derive(Debug, Clone, Copy)]
enum State {
    NotLoggedIn,
    LoggingIn,
    LoggedIn,
}

pub struct Client {
    state: State,
    tx: Sender<Request>,
    rx: Receiver<Response>,
}

impl Client {
    pub fn new(tx: Sender<Request>, rx: Receiver<Response>) -> Self {
        Client {
            state: State::NotLoggedIn,
            tx: tx,
            rx: rx,
        }
    }

    pub fn run(&mut self) {
        info!("Logging in...");
        self.state = State::LoggingIn;
        let server_request = ServerRequest::LoginRequest(LoginRequest::new(
                config::USERNAME,
                config::PASSWORD,
                config::VER_MAJOR,
                config::VER_MINOR,
                ).unwrap());
        self.tx.send(Request::ServerRequest(server_request)).unwrap();

        loop {
            let response = match self.rx.recv() {
                Ok(response) => response,
                Err(e) => {
                    error!("Error receiving response: {}", e);
                    break;
                },
            };
            match response {
                Response::ServerResponse(server_response) =>
                    self.handle_server_response(server_response),
            }
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
