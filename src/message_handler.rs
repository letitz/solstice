use std::io;

use crate::context::Context;

/// A trait for types that can handle reception of a message.
pub trait MessageHandler<Message> {
    fn run(self, context: &Context, message: Message) -> io::Result<()>;
}
