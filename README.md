# sharded-counter

\[ [docs.rs](https://docs.rs/sharded-counter/) | [crates.io](https://crates.io/crates/sharded-counter) \]

It is a concurrent counter recommended for the scenario where multiple
threads need to allocate unique IDs fast. It works by a global
`CounterPool` which is shared among the threads. Each thread creates a
`LocalCounter` from the pool, which allocates a portion of unique
numbers. The numbers can be iterated, and each time they are depleted,
the conuter allocates more new numbers from the global pool.

## Examples

```rust
use sharded_counter::CounterPool;
use std::{sync::Arc, thread};

let num_threads = thread::available_parallelism().unwrap().get();
let pool = Arc::new(CounterPool::new());

let handles: Vec<_> = (0..num_threads)
    .map(|_| {
        let pool = pool.clone();

        thread::spawn(move || {
            let _counts: Vec<_> = pool.counter().take(1_000_000).collect();
        })
    })
    .collect();

for handle in handles {
    handle.join().unwrap();
}
```

## License

MIT license.
