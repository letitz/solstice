//! This module defines the central message dispatcher to the client process.

use std::fmt::Debug;

use crate::executor::Job;
use crate::message_handler::MessageHandler;
use crate::proto::server::ServerResponse;

/// The type of messages dispatched by a dispatcher.
enum Message {
    ServerResponse(ServerResponse),
}

/// Pairs together a message and its handler as chosen by the dispatcher.
/// Implements Execute so as to be scheduled on an executor.
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
    H: Debug + Send + MessageHandler<M>,
{
    fn execute(self: Box<Self>) {
        if let Err(error) = self.handler.run(&self.message) {
            error!(
                "Error in handler {}: {:?}\nMessage: {:?}",
                H::name(),
                error,
                &self.message
            );
        }
    }
}

struct Dispatcher;

impl Dispatcher {
    fn new() -> Self {
        Self {}
    }

    fn dispatch(message: Message) -> Box<dyn Job> {
        panic!("Unimplemented")
    }
}
