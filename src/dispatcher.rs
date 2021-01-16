//! This module defines the central message dispatcher to the client process.

use std::fmt::Debug;

use crate::context::Context;
use crate::executor::Job;
use crate::handlers::{LoginHandler, SetPrivilegedUsersHandler};
use crate::message_handler::MessageHandler;
use crate::proto::server::ServerResponse;

/// The type of messages dispatched by a dispatcher.
#[derive(Debug)]
pub enum Message {
  ServerResponse(ServerResponse),
}

/// Pairs together a message and its handler as chosen by the dispatcher.
/// Implements Job so as to be scheduled on an executor.
struct DispatchedMessage<M, H> {
  message: M,
  handler: H,
}

impl<M, H> DispatchedMessage<M, H> {
  fn new(message: M, handler: H) -> Self {
    Self { message, handler }
  }
}

impl<M, H> Job for DispatchedMessage<M, H>
where
  M: Debug + Send,
  H: MessageHandler<M> + Send,
{
  fn execute(self: Box<Self>, context: &Context) {
    if let Err(error) = self.handler.run(context, &self.message) {
      error!(
        "Error in handler {}: {:?}\nMessage: {:?}",
        H::name(),
        error,
        &self.message
      );
    }
  }
}

/// The Dispatcher is in charge of mapping messages to their handlers.
pub struct Dispatcher;

impl Dispatcher {
  /// Returns a new dispatcher.
  pub fn new() -> Self {
    Self {}
  }

  /// Dispatches the given message by wrapping it with a handler.
  pub fn dispatch(&self, message: Message) -> Box<dyn Job> {
    match message {
      Message::ServerResponse(ServerResponse::LoginResponse(response)) => {
        Box::new(DispatchedMessage::new(response, LoginHandler::default()))
      }
      Message::ServerResponse(ServerResponse::PrivilegedUsersResponse(
        response,
      )) => Box::new(DispatchedMessage::new(
        response,
        SetPrivilegedUsersHandler::default(),
      )),
      _ => panic!("Unimplemented"),
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::proto::server;

  use super::*;

  #[test]
  fn dispatcher_privileged_users_response() {
    Dispatcher::new().dispatch(Message::ServerResponse(
      server::ServerResponse::PrivilegedUsersResponse(
        server::PrivilegedUsersResponse {
          users: vec!["foo".to_string(), "bar".to_string(), "baz".to_string()],
        },
      ),
    ));
  }

  #[test]
  fn dispatcher_login_response() {
    Dispatcher::new().dispatch(Message::ServerResponse(
      server::ServerResponse::LoginResponse(server::LoginResponse::LoginFail {
        reason: "bleep bloop".to_string(),
      }),
    ));
  }
}
