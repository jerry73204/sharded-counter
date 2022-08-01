//! A fast and concurrent counter using sharding.
//!
//! The counter is recommended for the scenario where multiple threads
//! need to allocate unique IDs fast and concurrently. It works by a
//! global [CounterPool] shared among the threads. Each thread creates
//! a [LocalCounter] from the pool, which allocates a portion of
//! unique numbers. The numbers can be iterated, and each time they
//! are depleted, the conuter allocates more new numbers from the
//! global pool.
//!
//! # Usage
//!
//! The usage is to start by a [CounterPool] shared among the
//! threads. Each thread calls [.counter()](CounterPool::counter) to
//! obtain a local counter. The counter itself is an itertor
//! generating unique numbers.
//!
//!
//! ```rust
//! use sharded_counter::CounterPool;
//! use std::{sync::Arc, thread};
//!
//! let num_threads = thread::available_parallelism().unwrap().get();
//! let pool = Arc::new(CounterPool::new());
//!
//! let handles: Vec<_> = (0..num_threads)
//!     .map(|_| {
//!         let pool = pool.clone();
//!
//!         thread::spawn(move || {
//!             let _counts: Vec<_> = pool.counter().take(1_000_000).collect();
//!         })
//!     })
//!     .collect();
//!
//! for handle in handles {
//!     handle.join().unwrap();
//! }
//! ```

use std::{
    fmt,
    sync::{Mutex, MutexGuard},
};

use once_cell::sync::Lazy;

const BASE_POW: u32 = 8;

/// A pool of counter shards.
pub struct CounterPool {
    alloc: Mutex<CountAllocation>,
    shards: Box<[Mutex<CounterCell>]>,
}

impl fmt::Debug for CounterPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CounterPool")
            .field("shards", &format!("[{} shards]", self.shards.len()))
            .finish()
    }
}

impl CounterPool {
    /// Creates a new counter pool with the default number of shards.
    ///
    /// The default number of shards is determined by
    /// [default_shard_amount].
    pub fn new() -> Self {
        Self::with_shard_amount(default_shard_amount())
    }

    /// Creates a new builder with the specified amount of shards.
    pub fn with_shard_amount(shard_amount: usize) -> Self {
        let shards: Vec<_> = (0..shard_amount)
            .map(|_| Mutex::new(CounterCell::default()))
            .collect();
        let alloc = Mutex::new(CountAllocation { amount: 0, nth: 0 });

        Self {
            shards: shards.into_boxed_slice(),
            alloc,
        }
    }

    /// Get a local counter.
    ///
    /// The returned counter holds an exclusive lock to one shard
    /// inside the parent pool. The maximum number of available
    /// counters is restricted by `shard_amount`.
    ///
    /// # Panics
    /// This counter panics if all shareds are locked.
    pub fn counter(&self) -> LocalCounter<'_> {
        self.try_counter().unwrap()
    }

    /// Try to get an available local counter.
    ///
    /// The returned counter holds an exclusive lock to one shard
    /// inside the parent pool. If no shards is available, it returns
    /// `None`. The maximum number of available counters is restricted
    /// by `shard_amount`.
    pub fn try_counter(&self) -> Option<LocalCounter<'_>> {
        self.shards
            .iter()
            .find_map(|mutex| mutex.try_lock().ok())
            .map(|guard| LocalCounter {
                alloc: &self.alloc,
                guard,
            })
    }
}

impl Default for CounterPool {
    fn default() -> Self {
        Self::new()
    }
}

/// An iterator of unique numbers.
///
/// The generated numbers are unique within the parent [CounterPool].
/// The numbers may not be consecutive.
pub struct LocalCounter<'a> {
    alloc: &'a Mutex<CountAllocation>,
    guard: MutexGuard<'a, CounterCell>,
}

impl<'a> Iterator for LocalCounter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let guard = &mut self.guard;

        if guard.curr == guard.max {
            let mut alloc = self.alloc.lock().unwrap();

            let size = 2usize.pow(alloc.nth as u32 + BASE_POW);
            alloc.nth += 1;

            let orig_amount = alloc.amount;
            alloc.amount += size;

            guard.curr = orig_amount;
            guard.max = alloc.amount;
        }

        let curr = guard.curr;
        guard.curr += 1;
        Some(curr)
    }
}

#[derive(Debug, Clone, Default)]
struct CounterCell {
    curr: usize,
    max: usize,
}

struct CountAllocation {
    amount: usize,
    nth: usize,
}

/// Computes the default number of shards.
pub fn default_shard_amount() -> usize {
    static COUNT: Lazy<usize> = Lazy::new(|| {
        (std::thread::available_parallelism().map_or(1, usize::from) * 4).next_power_of_two()
    });

    *COUNT
}
