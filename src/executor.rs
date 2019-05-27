//! This module provides a facade for an abstract concurrent job executor.
//!
//! Mostly here to insulate the rest of this crate from the exact details of
//! the executor implementation.

use threadpool;

/// Default number of threads spawned by Executor instances
const NUM_THREADS: usize = 8;

/// The trait of objects that can be run by an Executor.
pub trait Job: Send {
    fn execute(self: Box<Self>);
}

/// A concurrent job execution engine.
pub struct Executor {
    /// Executes the jobs.
    pool: threadpool::ThreadPool,
}

impl Executor {
    /// Builds a new executor with a default number of threads.
    pub fn new() -> Self {
        Self {
            pool: threadpool::Builder::new()
                .num_threads(NUM_THREADS)
                .thread_name("Executor".to_string())
                .build(),
        }
    }

    /// Schedules execution of the given job on this executor.
    pub fn schedule(&self, job: Box<dyn Job>) {
        self.pool.execute(move || job.execute());
    }

    /// Blocks until all scheduled jobs are executed.
    pub fn join(self) {
        self.pool.join();
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::{Arc, Barrier};

    use super::{Executor, Job};

    #[test]
    fn immediate_join() {
        Executor::new().join()
    }

    struct Waiter {
        pub barrier: Arc<Barrier>,
    }

    impl Job for Waiter {
        fn execute(self: Box<Self>) {
            self.barrier.wait();
        }
    }

    #[test]
    fn join_waits_for_all_jobs() {
        let executor = Executor::new();

        let barrier = Arc::new(Barrier::new(2));

        executor.schedule(Box::new(Waiter {
            barrier: barrier.clone(),
        }));
        executor.schedule(Box::new(Waiter {
            barrier: barrier.clone(),
        }));

        executor.join();
    }
}
