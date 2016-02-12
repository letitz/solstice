use crypto::md5::Md5;
use crypto::digest::Digest;

use proto::{Message, MessageCode, Peer};

const VER_MAJOR : u32 = 181;
const VER_MINOR : u32 = 100;

const USERNAME : &'static str = "abcdefgh";
// The password is not used for much, and sent unencrypted over the wire, so
// why not even check it in to git
const PASSWORD : &'static str = "ijklmnop";

#[derive(Debug, Clone, Copy)]
enum State {
    NotLoggedIn,
    LoggingIn,
    LoggedIn,
}

#[derive(Debug)]
pub struct ServerConnection {
    state: State,
}

impl ServerConnection {
    pub fn new() -> Self {
        ServerConnection {
            state: State::NotLoggedIn,
        }
    }

    pub fn make_login_message(&mut self) -> Message {
        let mut msg = Message::new(MessageCode::Login);

        msg.write_str(USERNAME).unwrap();
        msg.write_str(PASSWORD).unwrap();
        msg.write_u32(VER_MAJOR).unwrap();

        let userpass = USERNAME.to_string() + PASSWORD;
        msg.write_str(&Self::md5_str(&userpass)).unwrap();

        msg.write_u32(VER_MINOR).unwrap();

        msg
    }

    fn md5_str(string: &str) -> String {
        let mut hasher = Md5::new();
        hasher.input_str(string);
        hasher.result_str()
    }
}

impl Peer for ServerConnection {

    fn read_message(&mut self) -> Option<Message> {
        match self.state {
            State::NotLoggedIn => {
                println!("Logging in...");
                self.state = State::LoggingIn;
                Some(self.make_login_message())
            },
            _ => None
        }
    }

    fn write_message(&mut self, message: Message) {
        println!("write_message: {:?}", message);
        match self.state {
            State::LoggingIn => (),
            _ => ()
        }
    }
}
