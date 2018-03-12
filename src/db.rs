use std::ops::Deref;
use std::env;

use r2d2;
use r2d2_redis::RedisConnectionManager;

pub type Pool = r2d2::Pool<RedisConnectionManager>;

pub fn pool() -> Pool {
    let database_url: String = env::var("REDISCLOUD_URL").expect("$REDISCLOUD_URL is provided");
    info!("REDISCLOUD_URL: {:?}", database_url);
    let manager = RedisConnectionManager::new(&*database_url).unwrap();
    r2d2::Pool::builder()
        .build(manager)
        .expect("db pool is not created")
}

pub struct RedisConnection(pub r2d2::PooledConnection<RedisConnectionManager>);

impl Deref for RedisConnection {
    type Target = r2d2::PooledConnection<RedisConnectionManager>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
