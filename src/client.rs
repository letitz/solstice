use std::net;

use crossbeam_channel;
use mio;
use slab;

use crate::config;
use crate::control;
use crate::proto;
use crate::proto::peer;
use crate::proto::server;
use crate::room;
use crate::user;

#[derive(Debug)]
enum IncomingMessage {
    Proto(proto::Response),
    ControlNotification(control::Notification),
}

#[derive(Clone, Debug)]
enum LoginStatus {
    Pending,
    Success(String),
    Failure(String),
}

#[derive(Debug)]
enum PeerState {
    /// We are trying to establish a direct connection.
    Opening,
    /// We are trying to establish a reverse connection.
    OpeningFirewalled,
    /// We are waiting for a reverse connection to be established to us.
    WaitingFirewalled,
    /// The connection is open.
    Open,
}

#[derive(Debug)]
struct Peer {
    user_name: String,
    ip: net::Ipv4Addr,
    port: u16,
    connection_type: String,
    token: u32,
    state: PeerState,
}

pub struct Client {
    proto_tx: mio::deprecated::Sender<proto::Request>,
    proto_rx: crossbeam_channel::Receiver<proto::Response>,

    control_tx: Option<control::Sender>,
    control_rx: crossbeam_channel::Receiver<control::Notification>,

    login_status: LoginStatus,

    rooms: room::RoomMap,
    users: user::UserMap,

    peers: slab::Slab<Peer, usize>,
}

impl Client {
    /// Returns a new client that will communicate with the protocol agent
    /// through `proto_tx` and `proto_rx`, and with the controller agent
    /// through `control_rx`.
    pub fn new(
        proto_tx: mio::deprecated::Sender<proto::Request>,
        proto_rx: crossbeam_channel::Receiver<proto::Response>,
        control_rx: crossbeam_channel::Receiver<control::Notification>,
    ) -> Self {
        Client {
            proto_tx: proto_tx,
            proto_rx: proto_rx,

            control_tx: None,
            control_rx: control_rx,

            login_status: LoginStatus::Pending,

            rooms: room::RoomMap::new(),
            users: user::UserMap::new(),

            peers: slab::Slab::new(config::MAX_PEERS),
        }
    }

    /// Runs the client, potentially forever.
    pub fn run(&mut self) {
        info!("Logging in...");
        self.send_to_server(server::ServerRequest::LoginRequest(
            server::LoginRequest::new(
                config::USERNAME,
                config::PASSWORD,
                config::VER_MAJOR,
                config::VER_MINOR,
            )
            .unwrap(),
        ));

        self.send_to_server(server::ServerRequest::SetListenPortRequest(
            server::SetListenPortRequest {
                port: config::LISTEN_PORT,
            },
        ));

        loop {
            match self.recv() {
                IncomingMessage::Proto(response) => self.handle_proto_response(response),

                IncomingMessage::ControlNotification(notif) => {
                    self.handle_control_notification(notif)
                }
            }
        }
    }

    // Necessary to break out in different function because self cannot be
    // borrowed in the select arms due to *macro things*.
    fn recv(&mut self) -> IncomingMessage {
        IncomingMessage::Proto(self.proto_rx.recv().unwrap())
    }

    /// Send a request to the server.
    fn send_to_server(&self, request: server::ServerRequest) {
        self.proto_tx
            .send(proto::Request::ServerRequest(request))
            .unwrap();
    }

    /// Send a message to a peer.
    fn send_to_peer(&self, peer_id: usize, message: peer::Message) {
        self.proto_tx
            .send(proto::Request::PeerMessage(peer_id, message))
            .unwrap();
    }

    /// Send a response to the controller client.
    fn send_to_controller(&mut self, response: control::Response) {
        let result = match self.control_tx {
            None => {
                // Silently drop control requests when controller is
                // disconnected.
                return;
            }
            Some(ref mut control_tx) => control_tx.send(response),
        };
        // If we failed to send, we assume it means that the other end of the
        // channel has been dropped, i.e. the controller has disconnected.
        // It may be that mio has died on us, in which case we will never see
        // a controller again. If that happens, there would have probably been
        // a panic anyway, so we might never hit this corner case.
        if let Err(_) = result {
            info!("Controller has disconnected.");
            self.control_tx = None;
        }
    }

    /*===============================*
     * CONTROL NOTIFICATION HANDLING *
     *===============================*/

    fn handle_control_notification(&mut self, notif: control::Notification) {
        match notif {
            control::Notification::Connected(tx) => {
                self.control_tx = Some(tx);
            }

            control::Notification::Disconnected => {
                self.control_tx = None;
            }

            control::Notification::Error(e) => {
                debug!("Control loop error: {}", e);
                self.control_tx = None;
            }

            control::Notification::Request(req) => self.handle_control_request(req),
        }
    }

    /*==========================*
     * CONTROL REQUEST HANDLING *
     *==========================*/

    fn handle_control_request(&mut self, request: control::Request) {
        match request {
            control::Request::LoginStatusRequest => self.handle_login_status_request(),

            control::Request::RoomJoinRequest(room_name) => {
                self.handle_room_join_request(room_name)
            }

            control::Request::RoomLeaveRequest(room_name) => {
                self.handle_room_leave_request(room_name)
            }

            control::Request::RoomListRequest => self.handle_room_list_request(),

            control::Request::RoomMessageRequest(request) => {
                self.handle_room_message_request(request)
            }

            control::Request::UserListRequest => self.handle_user_list_request(),
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
            LoginStatus::Pending => control::LoginStatusResponse::Pending { username: username },

            LoginStatus::Success(ref motd) => control::LoginStatusResponse::Success {
                username: username,
                motd: motd.clone(),
            },

            LoginStatus::Failure(ref reason) => control::LoginStatusResponse::Failure {
                username: username,
                reason: reason.clone(),
            },
        };
        self.send_to_controller(control::Response::LoginStatusResponse(response));
    }

    fn handle_room_join_request(&mut self, room_name: String) {
        match self.rooms.start_joining(&room_name) {
            Ok(()) => {
                info!("Requesting to join room {:?}", room_name);
                self.send_to_server(server::ServerRequest::RoomJoinRequest(
                    server::RoomJoinRequest {
                        room_name: room_name,
                    },
                ));
            }

            Err(err) => error!("RoomLeaveRequest: {}", err),
        }
    }

    fn handle_room_leave_request(&mut self, room_name: String) {
        match self.rooms.start_leaving(&room_name) {
            Ok(()) => {
                info!("Requesting to leave room {:?}", room_name);
                self.send_to_server(server::ServerRequest::RoomLeaveRequest(
                    server::RoomLeaveRequest {
                        room_name: room_name,
                    },
                ));
            }

            Err(err) => error!("RoomLeaveRequest: {}", err),
        }
    }

    fn handle_room_list_request(&mut self) {
        // First send the controller client what we have in memory.
        let rooms = self.rooms.get_room_list();
        self.send_to_controller(control::Response::RoomListResponse(
            control::RoomListResponse { rooms: rooms },
        ));
        // Then ask the server for an updated version, which will be forwarded
        // to the controller client once received.
        self.send_to_server(server::ServerRequest::RoomListRequest);
    }

    fn handle_room_message_request(&mut self, request: control::RoomMessageRequest) {
        self.send_to_server(server::ServerRequest::RoomMessageRequest(
            server::RoomMessageRequest {
                room_name: request.room_name,
                message: request.message,
            },
        ));
    }

    fn handle_user_list_request(&mut self) {
        // Send the controller client what we have in memory.
        let user_list = self.users.get_list();
        self.send_to_controller(control::Response::UserListResponse(
            control::UserListResponse {
                user_list: user_list,
            },
        ));
    }

    /*=========================*
     * PROTO RESPONSE HANDLING *
     *=========================*/

    fn handle_proto_response(&mut self, response: proto::Response) {
        match response {
            proto::Response::ServerResponse(server_response) => {
                self.handle_server_response(server_response)
            }

            proto::Response::PeerConnectionOpen(peer_id) => {
                self.handle_peer_connection_open(peer_id)
            }

            proto::Response::PeerConnectionClosed(peer_id) => {
                self.handle_peer_connection_closed(peer_id)
            }

            _ => {
                warn!("Unhandled proto response: {:?}", response);
            }
        }
    }

    fn handle_peer_connection_closed(&mut self, peer_id: usize) {
        let mut occupied_entry = match self.peers.entry(peer_id) {
            None | Some(slab::Entry::Vacant(_)) => {
                error!("Unknown peer connection {} has closed", peer_id);
                return;
            }

            Some(slab::Entry::Occupied(occupied_entry)) => occupied_entry,
        };

        match occupied_entry.get_mut().state {
            PeerState::Open => {
                info!("Peer connection {} has closed", peer_id);
                occupied_entry.remove();
            }

            PeerState::WaitingFirewalled => {
                error!(
                    "Peer connection {} has closed, was waiting: inconsistent",
                    peer_id
                );
                occupied_entry.remove();
            }

            PeerState::Opening => {
                info!(
                    "Peer connection {} has been refused, trying reverse",
                    peer_id
                );

                let peer = occupied_entry.get_mut();
                peer.state = PeerState::WaitingFirewalled;

                self.proto_tx
                    .send(proto::Request::ServerRequest(
                        server::ServerRequest::ConnectToPeerRequest(server::ConnectToPeerRequest {
                            token: peer.token,
                            user_name: peer.user_name.clone(),
                            connection_type: peer.connection_type.clone(),
                        }),
                    ))
                    .unwrap();
            }

            PeerState::OpeningFirewalled => {
                info!(
                    "Peer connection {} has been refused, cannot connect",
                    peer_id
                );

                let (peer, _) = occupied_entry.remove();
                self.proto_tx
                    .send(proto::Request::ServerRequest(
                        server::ServerRequest::CannotConnectRequest(server::CannotConnectRequest {
                            token: peer.token,
                            user_name: peer.user_name,
                        }),
                    ))
                    .unwrap();
            }
        }
    }

    fn handle_peer_connection_open(&mut self, peer_id: usize) {
        let message = match self.peers.get_mut(peer_id) {
            None => {
                error!("Unknown peer connection {} is open", peer_id);
                return;
            }

            Some(
                peer @ &mut Peer {
                    state: PeerState::Open,
                    ..
                },
            ) => {
                error!("Peer connection {} was already open: {:?}", peer_id, peer);
                return;
            }

            Some(
                peer @ &mut Peer {
                    state: PeerState::WaitingFirewalled,
                    ..
                },
            ) => {
                error!("Peer connection {} was waiting: {:?}", peer_id, peer);
                return;
            }

            Some(
                peer @ &mut Peer {
                    state: PeerState::Opening,
                    ..
                },
            ) => {
                info!("Peer connection {} is now open: {:?}", peer_id, peer);
                // Mark it as open.
                peer.state = PeerState::Open;
                // Send a PeerInit.
                peer::Message::PeerInit(peer::PeerInit {
                    user_name: config::USERNAME.to_string(),
                    connection_type: peer.connection_type.clone(),
                    token: peer.token,
                })
            }

            Some(
                peer @ &mut Peer {
                    state: PeerState::OpeningFirewalled,
                    ..
                },
            ) => {
                info!("Peer connection {} is now open: {:?}", peer_id, peer);
                // Mark it as open.
                peer.state = PeerState::Open;
                // Send a PierceFirewall.
                peer::Message::PierceFirewall(peer.token)
            }
        };

        self.send_to_peer(peer_id, message);
    }

    /*==========================*
     * SERVER RESPONSE HANDLING *
     *==========================*/

    fn handle_server_response(&mut self, response: server::ServerResponse) {
        match response {
            server::ServerResponse::ConnectToPeerResponse(response) => {
                self.handle_connect_to_peer_response(response)
            }

            server::ServerResponse::LoginResponse(response) => self.handle_login_response(response),

            server::ServerResponse::PrivilegedUsersResponse(response) => {
                self.handle_privileged_users_response(response)
            }

            server::ServerResponse::RoomJoinResponse(response) => {
                self.handle_room_join_response(response)
            }

            server::ServerResponse::RoomLeaveResponse(response) => {
                self.handle_room_leave_response(response)
            }

            server::ServerResponse::RoomListResponse(response) => {
                self.handle_room_list_response(response)
            }

            server::ServerResponse::RoomMessageResponse(response) => {
                self.handle_room_message_response(response)
            }

            server::ServerResponse::RoomTickersResponse(response) => {
                self.handle_room_tickers_response(response)
            }

            server::ServerResponse::RoomUserJoinedResponse(response) => {
                self.handle_room_user_joined_response(response)
            }

            server::ServerResponse::RoomUserLeftResponse(response) => {
                self.handle_room_user_left_response(response)
            }

            server::ServerResponse::UserInfoResponse(response) => {
                self.handle_user_info_response(response)
            }

            server::ServerResponse::UserStatusResponse(response) => {
                self.handle_user_status_response(response)
            }

            server::ServerResponse::UnknownResponse(code) => {
                warn!("Unknown response: code {}", code)
            }

            response => warn!("Unhandled response: {:?}", response),
        }
    }

    fn handle_connect_to_peer_response(&mut self, response: server::ConnectToPeerResponse) {
        let peer = Peer {
            user_name: response.user_name,
            ip: response.ip,
            port: response.port,
            connection_type: response.connection_type,
            token: response.token,
            state: PeerState::OpeningFirewalled,
        };

        match self.peers.insert(peer) {
            Ok(peer_id) => {
                info!(
                    "Opening peer connection {} to {}:{} to pierce firewall",
                    peer_id, response.ip, response.port
                );
                self.proto_tx
                    .send(proto::Request::PeerConnect(
                        peer_id,
                        response.ip,
                        response.port,
                    ))
                    .unwrap();
            }

            Err(peer) => {
                warn!(
                    "Cannot open peer connection {:?}: too many already open",
                    peer
                );
            }
        }
    }

    fn handle_login_response(&mut self, login: server::LoginResponse) {
        if let LoginStatus::Pending = self.login_status {
            match login {
                server::LoginResponse::LoginOk {
                    motd,
                    ip,
                    password_md5_opt,
                } => {
                    info!("Login successful!");
                    info!("MOTD: \"{}\"", motd);
                    info!("External IP address: {}", ip);

                    match password_md5_opt {
                        Some(_) => {
                            info!(concat!(
                                "Connected to official server ",
                                "as official client"
                            ));
                        }
                        None => info!(concat!(
                            "Connected to official server ",
                            "as unofficial client"
                        )),
                    }
                    self.login_status = LoginStatus::Success(motd);
                }

                server::LoginResponse::LoginFail { reason } => {
                    error!("Login failed: \"{}\"", reason);
                    self.login_status = LoginStatus::Failure(reason);
                }
            }
        } else {
            error!(
                "Received unexpected login response, status = {:?}",
                self.login_status
            );
        }
    }

    fn handle_privileged_users_response(&mut self, response: server::PrivilegedUsersResponse) {
        self.users.set_all_privileged(response.users);
    }

    fn handle_room_join_response(&mut self, mut response: server::RoomJoinResponse) {
        // Join the room and store the received information.
        let result = self.rooms.join(
            &response.room_name,
            response.owner,
            response.operators,
            &response.users,
        );
        if let Err(err) = result {
            error!("RoomJoinResponse: {}", err);
            return;
        }

        // Then update the user structs based on the info we just got.
        for user in response.users.drain(..) {
            self.users.insert(user);
        }

        let control_response = control::RoomJoinResponse {
            room_name: response.room_name,
        };
        self.send_to_controller(control::Response::RoomJoinResponse(control_response));
    }

    fn handle_room_leave_response(&mut self, response: server::RoomLeaveResponse) {
        if let Err(err) = self.rooms.leave(&response.room_name) {
            error!("RoomLeaveResponse: {}", err);
        }

        self.send_to_controller(control::Response::RoomLeaveResponse(
            control::RoomLeaveResponse {
                room_name: response.room_name,
            },
        ));
    }

    fn handle_room_list_response(&mut self, response: server::RoomListResponse) {
        // Update the room map in memory.
        self.rooms.set_room_list(response);
        // Send the updated version to the controller.
        let rooms = self.rooms.get_room_list();
        self.send_to_controller(control::Response::RoomListResponse(
            control::RoomListResponse { rooms: rooms },
        ));
    }

    fn handle_room_message_response(&mut self, response: server::RoomMessageResponse) {
        let result = self.rooms.add_message(
            &response.room_name,
            room::Message {
                user_name: response.user_name.clone(),
                message: response.message.clone(),
            },
        );
        if let Err(err) = result {
            error!("RoomMessageResponse: {}", err);
            return;
        }

        self.send_to_controller(control::Response::RoomMessageResponse(
            control::RoomMessageResponse {
                room_name: response.room_name,
                user_name: response.user_name,
                message: response.message,
            },
        ));
    }

    fn handle_room_tickers_response(&mut self, response: server::RoomTickersResponse) {
        let result = self
            .rooms
            .set_tickers(&response.room_name, response.tickers);
        if let Err(e) = result {
            error!("RoomTickersResponse: {}", e);
        }
    }

    fn handle_room_user_joined_response(&mut self, response: server::RoomUserJoinedResponse) {
        let result = self
            .rooms
            .insert_member(&response.room_name, response.user.name.clone());
        if let Err(err) = result {
            error!("RoomUserJoinedResponse: {}", err);
            return;
        }
        self.send_to_controller(control::Response::RoomUserJoinedResponse(
            control::RoomUserJoinedResponse {
                room_name: response.room_name,
                user_name: response.user.name,
            },
        ));
    }

    fn handle_room_user_left_response(&mut self, response: server::RoomUserLeftResponse) {
        let result = self
            .rooms
            .remove_member(&response.room_name, &response.user_name);
        if let Err(err) = result {
            error!("RoomUserLeftResponse: {}", err);
            return;
        }
        self.send_to_controller(control::Response::RoomUserLeftResponse(
            control::RoomUserLeftResponse {
                room_name: response.room_name,
                user_name: response.user_name,
            },
        ));
    }

    fn handle_user_info_response(&mut self, response: server::UserInfoResponse) {
        let c_response = match self.users.get_mut_strict(&response.user_name) {
            Ok(user) => {
                user.average_speed = response.average_speed;
                user.num_downloads = response.num_downloads;
                user.num_files = response.num_files;
                user.num_folders = response.num_folders;
                control::UserInfoResponse {
                    user_name: response.user_name,
                    user_info: user.clone(),
                }
            }
            Err(err) => {
                error!("UserInfoResponse: {}", err);
                return;
            }
        };
        self.send_to_controller(control::Response::UserInfoResponse(c_response));
    }

    fn handle_user_status_response(&mut self, response: server::UserStatusResponse) {
        let result = self.users.set_status(&response.user_name, response.status);
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
