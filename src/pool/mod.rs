use std::ops::Deref;
use std::env;

use r2d2;
use r2d2_redis::RedisConnectionManager;

pub type Pool = r2d2::Pool<RedisConnectionManager>;

pub fn new_pool() -> Pool {
    let database_url: String = env::var("REDISCLOUD_URL").expect("$REDISCLOUD_URL is provided");
    let max_size: String = env::var("REDIS_CONNECTION_MAX_SIZE").expect("$REDIS_CONNECTION_MAX_SIZE is provided");
    info!("REDISCLOUD_URL: {:?}", database_url);
    info!("REDIS_CONNECTION_MAX_SIZE: {:?}", max_size);

    let manager = RedisConnectionManager::new(&*database_url).unwrap();
    r2d2::Pool::builder()
        .max_size(max_size.parse::<u32>().unwrap())
        .build(manager)
        .expect("db pool is not created")
}
