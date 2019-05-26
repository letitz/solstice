use std::io;
use std::sync::Arc;

use threadpool;

use crate::context::Context;

/// Default number of threads spawned by Executor instances
const NUM_THREADS: usize = 8;

/// The trait of objects that can be run by an Executor.
///
/// NOTE: Intended to be used by boxed objects, so that self's contents can be
/// moved by `execute` without running into "unknown size at compiled time"
/// E0161 errors.
pub trait Job: Send {
    /// Executes self in the given context.
    /// Errors do not crash the process, but are error-logged.
    fn execute(self: Box<Self>, context: &Context) -> io::Result<()>;
}

/// The central executor object that drives the client process.
pub struct Executor {
    /// The context against which jobs are executed.
    context: Arc<Context>,

    /// Executes the jobs.
    pool: threadpool::ThreadPool,
}

impl Executor {
    /// Builds a new executor with an empty context a default number of threads.
    pub fn new() -> Self {
        Self {
            context: Arc::new(Context::new()),
            pool: threadpool::Builder::new()
                .num_threads(NUM_THREADS)
                .thread_name("Executor".to_string())
                .build(),
        }
    }

    /// Schedules execution of the given job on this executor.
    pub fn schedule(&self, job: Box<dyn Job>) {
        let context = self.context.clone();
        self.pool.execute(move || {
            if let Err(error) = job.execute(&*context) {
                error!("Executable returned error: {:?}", error)
            }
        })
    }

    /// Blocks until all scheduled jobs are executed.
    pub fn join(self) {
        self.pool.join()
    }
}
