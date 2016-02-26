mod packet;
pub mod server;

pub use self::packet::{PacketStream, Packet};

use self::server::{ServerRequest, ServerResponse};

pub enum Request {
    ServerRequest(ServerRequest),
}

pub enum Response {
    ServerResponse(ServerResponse),
}
