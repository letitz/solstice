use std::sync::mpsc;

use mio;

use config;
use control::{ControlRequest, ControlResponse};
use proto::{Response, Request};
use proto::server::*;

#[derive(Debug)]
enum IncomingMessage {
    ServerResponse(ServerResponse),
    ControlRequest(ControlRequest),
}

#[derive(Debug, Clone, Copy)]
enum State {
    NotLoggedIn,
    LoggingIn,
    LoggedIn,
}

pub struct Client {
    state: State,

    proto_tx: mio::Sender<Request>,
    proto_rx: mpsc::Receiver<Response>,

    control_tx: mpsc::Sender<ControlResponse>,
    control_rx: mpsc::Receiver<ControlRequest>,
}

impl Client {
    pub fn new(
        proto_tx: mio::Sender<Request>,
        proto_rx: mpsc::Receiver<Response>,
        control_tx: mpsc::Sender<ControlResponse>,
        control_rx: mpsc::Receiver<ControlRequest>)
        -> Self
    {
        Client {
            state: State::NotLoggedIn,
            proto_tx: proto_tx,
            proto_rx: proto_rx,
            control_tx: control_tx,
            control_rx: control_rx,
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
        self.proto_tx.send(Request::ServerRequest(server_request)).unwrap();

        loop {
            match self.recv() {
                IncomingMessage::ServerResponse(response) =>
                    self.handle_server_response(response),

                IncomingMessage::ControlRequest(request) =>
                    self.handle_control_request(request),
            }
        }
    }

    fn recv(&mut self) -> IncomingMessage {
        let proto_rx = &self.proto_rx;
        let control_rx = &self.control_rx;
        select! {
            result = proto_rx.recv() =>
                match result.unwrap() {
                    Response::ServerResponse(server_response) =>
                        IncomingMessage::ServerResponse(server_response),
                },

            result = control_rx.recv() =>
                IncomingMessage::ControlRequest(result.unwrap())
        }
    }

    fn handle_control_request(&mut self, request: ControlRequest) {
        match request {
            _ => {
                error!("Unhandled control request: {:?}", request);
            },
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
