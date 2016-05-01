mod handler;
mod packet;
pub mod server;

pub use self::handler::*;

pub use self::packet::*;

use self::server::{ServerRequest, ServerResponse};

pub enum Request {
    ServerRequest(ServerRequest),
}

pub enum Response {
    ServerResponse(ServerResponse),
}
