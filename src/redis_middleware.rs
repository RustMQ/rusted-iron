extern crate futures;

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process;
use futures::{future, Future};

use gotham::middleware::{Middleware};
use gotham::state::State;
use gotham::handler::HandlerFuture;

use db::{Pool, RedisConnection};

#[derive(StateData)]
pub struct RedisMiddlewareData {
    pub connection: RedisConnection
}

#[derive(NewMiddleware)]
pub struct RedisMiddleware {
    pool: AssertUnwindSafe<Pool>
}

impl RedisMiddleware {
    pub fn new(pool: Pool) -> Self {
        RedisMiddleware {
            pool: AssertUnwindSafe(pool)
        }
    }
}

impl Middleware for RedisMiddleware {
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        let c = {
            let p = self.pool;
            p.get().unwrap()
        };
        let rc = RedisConnection(c);

        state.put(RedisMiddlewareData {
            connection: rc
        });

        let result = chain(state);

        let f = result.and_then(move |(state, mut response)| {
            future::ok((state, response))
        });

        Box::new(f)
    }
}

impl Clone for RedisMiddleware {
    fn clone(&self) -> Self {
        match catch_unwind(|| self.pool.clone()) {
            Ok(pool) => RedisMiddleware {
                pool: AssertUnwindSafe(pool),
            },
            Err(_) => {
                error!("PANIC: r2d2::Pool::clone caused a panic");
                eprintln!("PANIC: r2d2::Pool::clone caused a panic");
                process::abort()
            }
        }
    }
}
