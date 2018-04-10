extern crate futures;
extern crate tokio;
extern crate tokio_timer;

use tokio::prelude::*;
use tokio_timer::Interval;

use std::time::{Duration, Instant};

fn main() {
    let task = Interval::new(Instant::now(), Duration::from_millis(100))
        .take(10)
        .for_each(|instant| {
            println!("fire; instant={:?}", instant);
            Ok(())
        })
        .map_err(|e| panic!("interval errored; err={:?}", e));

    tokio::run(task);
}