use std::collections;
use std::sync::mpsc;

use mio;

use config;
use control;
use proto::{Response, Request};
use proto::server::*;
use room;

#[derive(Debug)]
enum IncomingMessage {
    ServerResponse(ServerResponse),
    ControlRequest(control::Request),
}

#[derive(Debug, Clone)]
enum LoginStatus {
    Pending,
    Success(String),
    Failure(String),
}

pub struct Client {
    proto_tx: mio::Sender<Request>,
    proto_rx: mpsc::Receiver<Response>,

    control_tx: mpsc::Sender<control::Response>,
    control_rx: mpsc::Receiver<control::Request>,

    login_status: LoginStatus,

    rooms: collections::HashMap<String, room::Room>,
    privileged_users: collections::HashSet<String>,
}

impl Client {
    pub fn new(
        proto_tx: mio::Sender<Request>,
        proto_rx: mpsc::Receiver<Response>,
        control_tx: mpsc::Sender<control::Response>,
        control_rx: mpsc::Receiver<control::Request>)
        -> Self
    {
        Client {
            proto_tx: proto_tx,
            proto_rx: proto_rx,
            control_tx: control_tx,
            control_rx: control_rx,

            login_status: LoginStatus::Pending,

            rooms: collections::HashMap::new(),
            privileged_users: collections::HashSet::new(),
        }
    }

    pub fn run(&mut self) {
        info!("Logging in...");
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

    /*==========================*
     * CONTROL REQUEST HANDLING *
     *==========================*/

    fn handle_control_request(&mut self, request: control::Request) {
        match request {
            control::Request::LoginStatusRequest =>
                self.handle_login_status_request(),

            control::Request::RoomListRequest =>
                self.handle_room_list_request(),

            control::Request::JoinRoomRequest(room_name) =>
                self.handle_join_room_request(room_name),

            /*
            _ =>{
                error!("Unhandled control request: {:?}", request);
            },
            */
        }
    }

    fn handle_join_room_request(&mut self, room_name: String) {
        let request = JoinRoomRequest { room_name: room_name };
        self.proto_tx.send(Request::ServerRequest(
                ServerRequest::JoinRoomRequest(request)));
    }

    fn handle_login_status_request(&mut self) {
        let username = config::USERNAME.to_string();

        let response = match self.login_status {
            LoginStatus::Pending =>
                control::LoginStatusResponse::Pending{ username: username },

            LoginStatus::Success(ref motd) =>
                control::LoginStatusResponse::Success{
                    username: username,
                    motd: motd.clone(),
                },

            LoginStatus::Failure(ref reason) =>
                control::LoginStatusResponse::Failure{
                    username: username,
                    reason: reason.clone(),
                },
        };
        self.control_tx.send(control::Response::LoginStatusResponse(response));
    }

    fn handle_room_list_request(&mut self) {
        let mut response = control::RoomListResponse{ rooms: Vec::new() };
        for (room_name, room) in self.rooms.iter() {
            response.rooms.push((room_name.clone(), room.clone()));
        }
        self.control_tx.send(control::Response::RoomListResponse(response));
    }

    /*==========================*
     * SERVER RESPONSE HANDLING *
     *==========================*/

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
        if let LoginStatus::Pending = self.login_status {
            match login {
                LoginResponse::LoginOk{ motd, ip, password_md5_opt } => {
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
                    self.login_status = LoginStatus::Success(motd);
                },

                LoginResponse::LoginFail{ reason } => {
                    error!("Login failed: \"{}\"", reason);
                    self.login_status = LoginStatus::Failure(reason);
                }
            }
        } else {
            error!("Received unexpected login response, status = {:?}",
                   self.login_status);
        }
    }

    fn handle_room_list_response(
        &mut self, mut response: RoomListResponse)
    {
        self.rooms.clear();
        for (name, user_count) in response.rooms.drain(..) {
            self.rooms.insert(name, room::Room{
                kind: room::RoomKind::Public,
                operated: false,
                user_count: user_count as usize,
            });
        }
        for (name, user_count) in response.owned_private_rooms.drain(..) {
            let room = room::Room {
                kind: room::RoomKind::PrivateOwned,
                operated: false,
                user_count: user_count as usize,
            };
            if let Some(_) = self.rooms.insert(name, room) {
                error!("Room is both normal and owned_private");
            }
        }
        for (name, user_count) in response.other_private_rooms.drain(..) {
            let room = room::Room {
                kind: room::RoomKind::PrivateOther,
                operated: false,
                user_count: user_count as usize,
            };
            if let Some(_) = self.rooms.insert(name, room) {
                error!("Room is both normal and other_private");
            }
        }
        for name in response.operated_private_room_names.drain(..) {
            match self.rooms.get_mut(&name) {
                None => error!("Room {} is operated but does not exist", name),
                Some(room) => room.operated = true,
            }
        }
    }

    fn handle_privileged_users_response(
        &mut self, mut response: PrivilegedUsersResponse)
    {
        self.privileged_users.clear();
        for username in response.users.drain(..) {
            self.privileged_users.insert(username);
        }
    }
}
