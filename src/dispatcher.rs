//! This module defines the central message dispatcher to the client process.

use std::io;

use crate::context::Context;
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

impl<M: Send, H: MessageHandler<M> + Send> Job for DispatchedMessage<M, H> {
    fn execute(self: Box<Self>, context: &Context) -> io::Result<()> {
        self.handler.run(context, self.message)
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
