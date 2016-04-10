use std::collections;
use std::sync::mpsc;

use mio;

use config;
use control;
use proto::{Response, Request};
use proto::server::*;
use room;
use user;

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
    controller_connected: bool,

    login_status: LoginStatus,

    rooms: room::RoomMap,
    users: user::UserMap,
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
            controller_connected: false,

            login_status: LoginStatus::Pending,

            rooms: room::RoomMap::new(),
            users: user::UserMap::new(),
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
        self.server_send(server_request);

        loop {
            match self.recv() {
                IncomingMessage::ServerResponse(response) =>
                    self.handle_server_response(response),

                IncomingMessage::ControlRequest(request) =>
                    self.handle_control_request(request),
            }
        }
    }

    // Necessary to break out in different function because self cannot be
    // borrowed in the select arms due to *macro things*.
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

    /// Send a request to the server.
    fn server_send(&self, request: ServerRequest) {
        self.proto_tx.send(Request::ServerRequest(request)).unwrap();
    }

    /// Send a response to the controller client.
    fn control_send(&self, response: control::Response) {
        if !self.controller_connected {
            return; // Silently drop control packets when no-one is listening.
        }
        self.control_tx.send(response).unwrap();
    }

    /*==========================*
     * CONTROL REQUEST HANDLING *
     *==========================*/

    fn handle_control_request(&mut self, request: control::Request) {
        match request {
            control::Request::ConnectNotification => {
                info!("Controller client connected");
                self.controller_connected = true;
            },

            control::Request::DisconnectNotification => {
                info!("Controller client disconnected");
                self.controller_connected = false;
            },

            control::Request::JoinRoomRequest(room_name) =>
                self.handle_join_room_request(room_name),

            control::Request::LoginStatusRequest =>
                self.handle_login_status_request(),

            control::Request::RoomListRequest =>
                self.handle_room_list_request(),

            control::Request::SayRoomRequest(request) =>
                self.handle_say_room_request(request),

            /*
            _ =>{
                error!("Unhandled control request: {:?}", request);
            },
            */
        }
    }

    fn handle_join_room_request(&mut self, room_name: String) {
        // First check that we are not already a member.
        // Enclosed in a block otherwise the mutable borrow of self.rooms
        // prevents from calling self.server_send.
        {
            let room = match self.rooms.get_mut(&room_name) {
                Some(room) => room,
                None => {
                    error!("Cannot join inexistent room \"{}\"", room_name);
                    return;
                }
            };
            match room.membership {
                room::Membership::NonMember => {
                    // As expected.
                    room.membership = room::Membership::Joining;
                },

                room::Membership::Member => {
                    warn!(
                        "Controller requested to re-join room \"{}\"",
                        room_name
                    );
                    // TODO Send control response
                },

                membership => {
                    error!(
                        "Cannot join room \"{}\": current membership: {:?}",
                        room_name, membership
                    );
                }
            }
        }
        self.server_send(ServerRequest::JoinRoomRequest(JoinRoomRequest {
            room_name: room_name
        }));
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
        self.control_send(control::Response::LoginStatusResponse(response));
    }

    fn handle_room_list_request(&mut self) {
        // First send the controller client what we have in memory.
        let response = self.rooms.get_room_list_response();
        self.control_send(control::Response::RoomListResponse(response));
        // Then ask the server for an updated version, which will be forwarded
        // to the controller client once received.
        self.server_send(ServerRequest::RoomListRequest);
    }

    fn handle_say_room_request(
        &mut self, request: control::SayRoomRequest)
    {
        self.server_send(ServerRequest::SayRoomRequest(SayRoomRequest {
            room_name: request.room_name,
            message:   request.message,
        }));
    }

    /*==========================*
     * SERVER RESPONSE HANDLING *
     *==========================*/

    fn handle_server_response(&mut self, response: ServerResponse) {
        match response {
            ServerResponse::JoinRoomResponse(response) =>
                self.handle_join_room_response(response),

            ServerResponse::LoginResponse(response) =>
                self.handle_login_response(response),

            ServerResponse::PrivilegedUsersResponse(response) =>
                self.handle_privileged_users_response(response),

            ServerResponse::RoomListResponse(response) =>
                self.handle_room_list_response(response),

            ServerResponse::SayRoomResponse(response) =>
                self.handle_say_room_response(response),

            ServerResponse::UserJoinedRoomResponse(response) =>
                self.handle_user_joined_room_response(response),

            ServerResponse::UnknownResponse(code) =>
                warn!("Unknown response: code {}", code),

            response => warn!("Unhandled response: {:?}", response),
        }
    }

    fn handle_join_room_response(&mut self, mut response: JoinRoomResponse) {
        // First look up the room struct.
        let room = match self.rooms.get_mut(&response.room_name) {
            Some(room) => room,
            None => {
                error!(
                    "Received JoinRoomResponse for inexistent room \"{}\"",
                    &response.room_name
                );
                return;
            }
        };

        // Log what's happening.
        if let room::Membership::Joining = room.membership {
            info!("Joined room \"{}\"", response.room_name);
        } else {
            warn!(
                "Joined room \"{}\" but membership was already {:?}",
                response.room_name, room.membership
            );
        }

        // Update the room struct.
        room.membership = room::Membership::Member;
        room.user_count = response.users.len();
        room.owner      = response.owner;
        for user_name in response.operators.drain(..) {
            room.operators.insert(user_name.clone());
        }
        for &(ref user_name, _) in response.users.iter() {
            room.members.insert(user_name.clone());
        }

        // Then update the user structs based on the info we just got.
        for (name, user) in response.users.drain(..) {
            self.users.insert(name, user);
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

    fn handle_privileged_users_response(
        &mut self, mut response: PrivilegedUsersResponse)
    {
        self.users.update_privileges(response);
    }

    fn handle_room_list_response(&mut self, response: RoomListResponse) {
        // Update the room map in memory.
        self.rooms.update(response);
        // Send the updated version to the controller.
        let control_response = self.rooms.get_room_list_response();
        self.control_send(
            control::Response::RoomListResponse(control_response));
    }

    fn handle_say_room_response(&mut self, response: SayRoomResponse) {
        // TODO Save the message somewhere.
        let control_response = control::SayRoomResponse {
            room_name: response.room_name,
            user_name: response.user_name,
            message:   response.message,
        };
        self.control_send(control::Response::SayRoomResponse(control_response));
    }

    fn handle_user_joined_room_response(
        &mut self, mut response: UserJoinedRoomResponse)
    {
        if let Some(room) = self.rooms.get_mut(&response.room_name) {
            room.members.insert(response.user_name.clone());
            self.users.insert(response.user_name, response.user);
        }
    }
}
