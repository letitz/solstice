use std::io;

use crate::context::Context;
use crate::message_handler::MessageHandler;
use crate::proto::server::PrivilegedUsersResponse;

#[derive(Debug, Default)]
pub struct SetPrivilegedUsersHandler;

impl MessageHandler<PrivilegedUsersResponse> for SetPrivilegedUsersHandler {
    fn run(
        self,
        context: &Context,
        message: &PrivilegedUsersResponse,
    ) -> io::Result<()> {
        let users = message.users.clone();
        context.users.lock().set_all_privileged(users);
        Ok(())
    }

    fn name() -> String {
        "SetPrivilegedUsersHandler".to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::context::Context;
    use crate::message_handler::MessageHandler;
    use crate::proto::server::PrivilegedUsersResponse;

    use super::SetPrivilegedUsersHandler;

    #[test]
    fn run_sets_privileged_users() {
        let context = Context::new();

        let response = PrivilegedUsersResponse {
            users: vec![
                "aomame".to_string(),
                "billybob".to_string(),
                "carlos".to_string(),
            ],
        };

        SetPrivilegedUsersHandler::default()
            .run(&context, &response)
            .unwrap();

        let mut privileged = context.users.lock().get_all_privileged();
        privileged.sort();

        assert_eq!(privileged, response.users);
    }
}
