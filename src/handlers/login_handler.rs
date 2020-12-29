use std::io;

use crate::context::Context;
use crate::login::LoginStatus;
use crate::message_handler::MessageHandler;
use crate::proto::server::LoginResponse;

#[derive(Debug, Default)]
pub struct LoginHandler;

impl MessageHandler<LoginResponse> for LoginHandler {
    fn run(self, context: &Context, _message: &LoginResponse) -> io::Result<()> {
        let lock = context.login.lock();

        match *lock {
            LoginStatus::AwaitingResponse => (),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("unexpected login response, status = {:?}", *lock),
                ));
            }
        };

        unimplemented!();
    }

    fn name() -> String {
        "LoginHandler".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn run_fails_on_wrong_status() {
        let context = Context::new();

        let response = LoginResponse::LoginFail {
            reason: "bleep bloop".to_string(),
        };

        LoginHandler::default().run(&context, &response).unwrap();
    }
}
