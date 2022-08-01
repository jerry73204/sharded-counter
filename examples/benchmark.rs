use clap::Parser;
use sharded_counter::CounterPool;
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicUsize, Ordering::*},
        Arc,
    },
    thread,
    time::Instant,
};

#[derive(Parser)]
struct Opts {
    pub num_counts_per_thread: usize,
    #[clap(default_value_t = thread::available_parallelism().unwrap().get())]
    pub num_threads: usize,
}

pub fn main() {
    let opts = Opts::parse();

    atomic_counter_test(&opts);
    sharded_counter_test(&opts);
}

fn atomic_counter_test(opts: &Opts) {
    let counter = Arc::new(AtomicUsize::new(0));
    let num_rounds = opts.num_counts_per_thread;
    let num_threads = opts.num_threads;

    let since = Instant::now();
    let handles: Vec<_> = (0..opts.num_threads)
        .map(|_| {
            let counter = counter.clone();

            thread::spawn(move || {
                let numbers: Vec<_> = (0..num_rounds)
                    .map(|_| counter.fetch_add(1, Relaxed))
                    .collect();
                numbers
            })
        })
        .collect();

    let numbers: HashSet<_> = handles
        .into_iter()
        .flat_map(|handle| handle.join().unwrap())
        .collect();
    println!("atomic counter: {:?}", since.elapsed());

    assert_eq!(numbers.len(), num_threads * num_rounds);
}

fn sharded_counter_test(opts: &Opts) {
    let pool = Arc::new(CounterPool::new());
    let num_rounds = opts.num_counts_per_thread;
    let num_threads = opts.num_threads;

    let since = Instant::now();
    let handles: Vec<_> = (0..opts.num_threads)
        .map(|_| {
            let pool = pool.clone();

            thread::spawn(move || {
                let numbers: Vec<_> = pool.counter().take(num_rounds).collect();
                numbers
            })
        })
        .collect();

    let numbers: HashSet<_> = handles
        .into_iter()
        .flat_map(|handle| handle.join().unwrap())
        .collect();
    println!("sharded counter: {:?}", since.elapsed());

    assert_eq!(numbers.len(), num_threads * num_rounds);
}
