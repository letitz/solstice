//! This module provides a facade for an abstract concurrent job executor.
//!
//! Mostly here to insulate the rest of this crate from the exact details of
//! the executor implementation, though it also owns the process-wide context
//! data structure against which handlers are run.

use std::sync::Arc;

use threadpool;

use crate::context::Context;

/// Default number of threads spawned by Executor instances
const NUM_THREADS: usize = 8;

/// The trait of objects that can be run by an Executor.
pub trait Job: Send {
  /// Runs this job against the given context.
  fn execute(self: Box<Self>, context: &Context);
}

/// A concurrent job execution engine.
pub struct Executor {
  /// The context against which jobs are executed.
  context: Arc<Context>,

  /// Executes the jobs.
  pool: threadpool::ThreadPool,
}

impl Executor {
  /// Builds a new executor against the given context.
  pub fn new(context: Context) -> Self {
    Self {
      context: Arc::new(context),
      pool: threadpool::Builder::new()
        .num_threads(NUM_THREADS)
        .thread_name("Executor".to_string())
        .build(),
    }
  }

  /// Schedules execution of the given job on this executor.
  pub fn schedule(&self, job: Box<dyn Job>) {
    let context = self.context.clone();
    self.pool.execute(move || job.execute(&*context));
  }

  /// Blocks until all scheduled jobs are executed, then returns the context.
  pub fn join(self) -> Context {
    self.pool.join();

    // The only copies of the Arc are passed to the closures executed on
    // the threadpool. Once the pool is join()ed, there cannot exist any
    // other copies than ours, so we are safe to unwrap() the Arc.
    Arc::try_unwrap(self.context).unwrap()
  }
}

#[cfg(test)]
mod tests {
  use std::sync::{Arc, Barrier};

  use crate::proto::{User, UserStatus};

  use super::{Context, Executor, Job};

  #[test]
  fn immediate_join_returns_empty_context() {
    let context = Executor::new(Context::new()).join();
    assert_eq!(context.users.lock().get_list(), vec![]);
    assert_eq!(context.rooms.lock().get_room_list(), vec![]);
  }

  struct Waiter {
    barrier: Arc<Barrier>,
  }

  impl Job for Waiter {
    fn execute(self: Box<Self>, _context: &Context) {
      self.barrier.wait();
    }
  }

  #[test]
  fn join_waits_for_all_jobs() {
    let executor = Executor::new(Context::new());

    let barrier = Arc::new(Barrier::new(2));

    executor.schedule(Box::new(Waiter {
      barrier: barrier.clone(),
    }));
    executor.schedule(Box::new(Waiter {
      barrier: barrier.clone(),
    }));

    executor.join();
  }

  struct UserAdder {
    pub user: User,
  }

  impl Job for UserAdder {
    fn execute(self: Box<Self>, context: &Context) {
      context.users.lock().insert(self.user);
    }
  }

  #[test]
  fn jobs_access_context() {
    let executor = Executor::new(Context::new());

    let user1 = User {
      name: "potato".to_string(),
      status: UserStatus::Offline,
      average_speed: 0,
      num_downloads: 0,
      unknown: 0,
      num_files: 0,
      num_folders: 0,
      num_free_slots: 0,
      country: "YO".to_string(),
    };

    let mut user2 = user1.clone();
    user2.name = "rutabaga".to_string();

    executor.schedule(Box::new(UserAdder {
      user: user1.clone(),
    }));
    executor.schedule(Box::new(UserAdder {
      user: user2.clone(),
    }));

    let context = executor.join();

    let expected_users =
      vec![(user1.name.clone(), user1), (user2.name.clone(), user2)];

    let mut users = context.users.lock().get_list();
    users.sort();

    assert_eq!(users, expected_users);
  }
}
