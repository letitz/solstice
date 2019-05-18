use std::io;

use crate::context::Context;

/// The trait of objects that can be run by an Executor.
pub trait Execute {
    fn execute(self, context: &Context) -> io::Result<()>;
}

pub struct Executor {
    // TODO
}

impl Executor {
    pub fn new() -> Self { Self {} }

    pub fn enqueue(work: Box<dyn Execute>) {
        // TODO
    }
}
