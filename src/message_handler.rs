use std::fmt::Debug;
use std::io;

/// A trait for types that can handle reception of a message.
///
/// Message types are mapped to handler types by Dispatcher.
/// This trait is intended to allow composing handler logic.
pub trait MessageHandler<Message>: Debug {
    /// Attempts to handle the given message.
    fn run(self, message: &Message) -> io::Result<()>;

    /// Returns the name of this handler type.
    fn name() -> String;
}
