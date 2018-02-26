use std::ops::Deref;
use std::env;

use r2d2;
use redis::Connection;
use r2d2_redis::RedisConnectionManager;

use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Request, State, Outcome};

pub type Pool = r2d2::Pool<RedisConnectionManager>;

pub fn init_pool() -> Pool {
    let database_url = env::var("REDISCLOUD_URL").unwrap();
    println!("REDISCLOUD_URL: {:?}", database_url);
    let manager = RedisConnectionManager::new(&*database_url).unwrap();
    r2d2::Pool::builder()
        .build(manager)
        .expect("db pool is not created")
}

pub struct Conn(pub r2d2::PooledConnection<RedisConnectionManager>);

impl Deref for Conn {
    type Target = Connection;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for Conn {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Conn, ()> {
        let pool = request.guard::<State<Pool>>()?;
        match pool.get() {
            Ok(conn) => Outcome::Success(Conn(conn)),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ()))
        }
    }
}