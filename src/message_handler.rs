use std::io;

use crate::context::Context;

/// A trait for types that can handle reception of a message.
///
/// Message types are mapped to handler types by Dispatcher.
/// This trait is intended to allow composing handler logic.
pub trait MessageHandler<Message> {
    fn run(self, context: &Context, message: Message) -> io::Result<()>;
}
