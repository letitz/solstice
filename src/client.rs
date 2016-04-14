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

            control::Request::LoginStatusRequest =>
                self.handle_login_status_request(),

            control::Request::RoomJoinRequest(room_name) =>
                self.handle_room_join_request(room_name),

            control::Request::RoomLeaveRequest(room_name) =>
                self.handle_room_leave_request(room_name),

            control::Request::RoomListRequest =>
                self.handle_room_list_request(),

            control::Request::RoomMessageRequest(request) =>
                self.handle_room_message_request(request),

            /*
            _ =>{
                error!("Unhandled control request: {:?}", request);
            },
            */
        }
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

    fn handle_room_join_request(&mut self, room_name: String) {
        match self.rooms.start_joining(&room_name) {
            Ok(()) => {
                info!("Requesting to join room {:?}", room_name);
                self.server_send(
                    ServerRequest::RoomJoinRequest(RoomJoinRequest {
                        room_name: room_name
                    })
                );
            },

            Err(err) => error!("RoomLeaveRequest: {}", err)
        }
    }

    fn handle_room_leave_request(&mut self, room_name: String) {
        match self.rooms.start_leaving(&room_name) {
            Ok(()) => {
                info!("Requesting to leave room {:?}", room_name);
                self.server_send(
                    ServerRequest::RoomLeaveRequest(RoomLeaveRequest {
                        room_name: room_name
                    })
                );
            },

            Err(err) => error!("RoomLeaveRequest: {}", err)
        }
    }

    fn handle_room_list_request(&mut self) {
        // First send the controller client what we have in memory.
        let response = control::RoomListResponse {
            rooms: self.rooms.get_room_list(),
        };
        self.control_send(control::Response::RoomListResponse(response));
        // Then ask the server for an updated version, which will be forwarded
        // to the controller client once received.
        self.server_send(ServerRequest::RoomListRequest);
    }

    fn handle_room_message_request(
        &mut self, request: control::RoomMessageRequest)
    {
        self.server_send(ServerRequest::RoomMessageRequest(RoomMessageRequest {
            room_name: request.room_name,
            message:   request.message,
        }));
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

            ServerResponse::RoomJoinResponse(response) =>
                self.handle_room_join_response(response),

            ServerResponse::RoomLeaveResponse(response) =>
                self.handle_room_leave_response(response),

            ServerResponse::RoomListResponse(response) =>
                self.handle_room_list_response(response),

            ServerResponse::RoomMessageResponse(response) =>
                self.handle_room_message_response(response),

            ServerResponse::RoomUserJoinedResponse(response) =>
                self.handle_room_user_joined_response(response),

            ServerResponse::RoomUserLeftResponse(response) =>
                self.handle_room_user_left_response(response),

            ServerResponse::UserStatusResponse(response) =>
                self.handle_user_status_response(response),

            ServerResponse::UnknownResponse(code) =>
                warn!("Unknown response: code {}", code),

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

    fn handle_privileged_users_response(
        &mut self, response: PrivilegedUsersResponse)
    {
        self.users.set_all_privileged(response.users);
    }

    fn handle_room_join_response(&mut self, mut response: RoomJoinResponse) {
        // Join the room and store the received information.
        let result = self.rooms.join(
            &response.room_name, response.owner, response.operators,
            &response.users
        );
        if let Err(err) = result {
            error!("RoomJoinResponse: {}", err);
            return;
        }

        // Then update the user structs based on the info we just got.
        for (name, user) in response.users.drain(..) {
            self.users.insert(name, user);
        }

        let control_response = control::RoomJoinResponse {
            room_name: response.room_name
        };
        self.control_send(control::Response::RoomJoinResponse(
                control_response
        ));
    }

    fn handle_room_leave_response(&mut self, response: RoomLeaveResponse) {
        if let Err(err) = self.rooms.leave(&response.room_name) {
            error!("RoomLeaveResponse: {}", err);
        }

        self.control_send(control::Response::RoomLeaveResponse(
                response.room_name
        ));
    }

    fn handle_room_list_response(&mut self, response: RoomListResponse) {
        // Update the room map in memory.
        self.rooms.set_room_list(response);
        // Send the updated version to the controller.
        let control_response = control::RoomListResponse {
            rooms: self.rooms.get_room_list(),
        };
        self.control_send(
            control::Response::RoomListResponse(control_response));
    }

    fn handle_room_message_response(&mut self, response: RoomMessageResponse) {
        let result = self.rooms.add_message(&response.room_name, room::Message {
            user_name: response.user_name.clone(),
            message:   response.message.clone(),
        });
        if let Err(err) = result {
            error!("RoomMessageResponse: {}", err);
            return;
        }

        let control_response = control::RoomMessageResponse {
            room_name: response.room_name,
            user_name: response.user_name,
            message:   response.message,
        };
        self.control_send(
            control::Response::RoomMessageResponse(control_response));
    }

    fn handle_room_user_joined_response(
        &mut self, response: RoomUserJoinedResponse)
    {
        let result = self.rooms.insert_member(
            &response.room_name, response.user_name.clone()
        );
        match result {
            Ok(()) => {
                self.users.insert(response.user_name, response.user);
                // TODO send control response
            },
            Err(err) => error!("RoomUserJoinedResponse: {}", err)
        }
    }

    fn handle_room_user_left_response(
        &mut self, response: RoomUserLeftResponse)
    {
        let result = self.rooms.remove_member(
            &response.room_name, &response.user_name
        );
        match result {
            Ok(()) => {
                // TODO send control response
            },
            Err(err) => error!("RoomUserLeftResponse: {}", err)
        }
    }

    fn handle_user_status_response(&mut self, response: UserStatusResponse) {
        let result = self.users.set_status(
            &response.user_name, response.status
        );
        if let Err(err) = result {
            error!("UserStatusResponse: {}", err);
            return;
        }

        if response.is_privileged {
            self.users.insert_privileged(response.user_name);
        } else {
            self.users.remove_privileged(&response.user_name);
        }
    }
}
