pub const VER_MAJOR: u32 = 181;
pub const VER_MINOR: u32 = 100;

pub const USERNAME: &'static str = "abcdefgh";
// The password is not used for much, and sent unencrypted over the wire, so
// why not even check it in to git
pub const PASSWORD: &'static str = "ijklmnop";

pub const SERVER_HOST : &'static str = "server.slsknet.org";
pub const SERVER_PORT : u16 = 2242;

pub const LISTEN_HOST: &'static str = "0.0.0.0";
pub const LISTEN_PORT: u16 = 2243;

pub const CONTROL_HOST: &'static str = "localhost";
pub const CONTROL_PORT: u16 = 2244;

pub const MAX_PEERS: usize = 1000;
