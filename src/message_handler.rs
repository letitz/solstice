use std::fmt::Debug;
use std::io;

use crate::context::Context;

/// A trait for types that can handle reception of a message.
///
/// Message types are mapped to handler types by Dispatcher.
/// This trait is intended to allow composing handler logic.
pub trait MessageHandler<Message> {
    /// Attempts to handle the given message against the given context.
    fn run(self, context: &Context, message: &Message) -> io::Result<()>;

    /// Returns the name of this handler type.
    fn name() -> String;
}
