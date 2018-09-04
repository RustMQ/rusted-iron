extern crate num_cpus;

use std::env;
use scheduled_thread_pool::ScheduledThreadPool;
use std::sync::Arc;
use r2d2_redis::{r2d2, RedisConnectionManager};

pub type Pool = r2d2::Pool<RedisConnectionManager>;

pub fn new_pool() -> Pool {
    let database_url: String = env::var("REDISCLOUD_URL").expect("$REDISCLOUD_URL is provided");
    let max_size: String = env::var("REDIS_CONNECTION_MAX_SIZE").expect("$REDIS_CONNECTION_MAX_SIZE is provided");
    info!("REDISCLOUD_URL: {:?}", database_url);
    info!("REDIS_CONNECTION_MAX_SIZE: {:?}", max_size);
    let threads = num_cpus::get();
    let thread_pool = ScheduledThreadPool::new(threads);

    let manager = RedisConnectionManager::new(&*database_url).unwrap();
    r2d2::Pool::builder()
        .max_size(max_size.parse::<u32>().unwrap())
        .thread_pool(Arc::new(thread_pool))
        .build(manager)
        .expect("db pool is not created")
}
