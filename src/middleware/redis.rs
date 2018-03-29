extern crate futures;
extern crate gotham;

extern crate redis;
extern crate r2d2;
extern crate r2d2_redis;

use std::{
    io,
    panic::{
        catch_unwind,
        AssertUnwindSafe
    },
    process
};

use futures::{future, Future};

use gotham::{
    handler::HandlerFuture,
    middleware::{
        Middleware,
        NewMiddleware
    },
    state::{
        request_id,
        State
    }
};
use r2d2_redis::RedisConnectionManager;

#[derive(StateData)]
pub struct RedisPool {
    pub pool: r2d2::Pool<RedisConnectionManager>,
}

impl RedisPool {

    pub fn new(pool: r2d2::Pool<RedisConnectionManager>) -> Self {
        RedisPool { pool }
    }

    pub fn conn(&self) -> Result<r2d2::PooledConnection<RedisConnectionManager>, r2d2::Error> {
        self.pool.get()
    }
}

pub struct RedisMiddleware {
    pool: AssertUnwindSafe<r2d2::Pool<RedisConnectionManager>>,
}

pub struct RedisMiddlewareImpl {
    pool: r2d2::Pool<RedisConnectionManager>,
}

impl RedisMiddleware
{
    pub fn with_pool(pool: r2d2::Pool<RedisConnectionManager>) -> Self {
        RedisMiddleware {
            pool: AssertUnwindSafe(pool),
        }
    }
}

impl Middleware for RedisMiddlewareImpl {
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        trace!("[{}] pre chain", request_id(&state));
        state.put(RedisPool::new(self.pool));

        let f = chain(state).and_then(move |(state, response)| {
            {
                trace!("[{}] post chain", request_id(&state));
            }
            future::ok((state, response))
        });
        Box::new(f)
    }
}

impl NewMiddleware for RedisMiddleware {

    type Instance = RedisMiddlewareImpl;

    fn new_middleware(&self) -> io::Result<Self::Instance> {
        match catch_unwind(|| self.pool.clone()) {
            Ok(pool) => Ok(RedisMiddlewareImpl { pool }),
            Err(_) => {
                error!(
                    "PANIC: r2d2::Pool::clone caused a panic, unable to rescue with a HTTP error"
                );
                eprintln!(
                    "PANIC: r2d2::Pool::clone caused a panic, unable to rescue with a HTTP error"
                );
                process::abort()
            }
        }
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
