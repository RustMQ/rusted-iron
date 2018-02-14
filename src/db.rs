use std::ops::Deref;

use r2d2;
use redis::Connection;
use r2d2_redis::RedisConnectionManager;

use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Request, State, Outcome};

pub type Pool = r2d2::Pool<RedisConnectionManager>;

pub const DATABASE_URL: Option<&'static str> = option_env!("REDISCLOUD_URL");

pub fn init_pool() -> Pool {
    println!("REDISCLOUD_URL: {:?}", DATABASE_URL);
    let manager: RedisConnectionManager = RedisConnectionManager::new(DATABASE_URL.expect("env is not path")).expect("manager created");
    r2d2::Pool::builder().build(manager).expect("db pool")
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