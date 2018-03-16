extern crate serde_json;
extern crate mime;
extern crate redis;

use redis::{InfoDict};
use hyper::{Response, StatusCode};
use gotham::state::{FromState, State};
use gotham::http::response::create_response;
use redis_middleware2::RedisPool;

pub fn version(mut state: State) -> (State, Response) {
    let rv = {
        let connection = {
            let redis_pool = RedisPool::borrow_mut_from(&mut state);
            let connection = redis_pool.conn().unwrap();
            connection
        };
        let info : InfoDict = redis::cmd("INFO").query(&*connection).unwrap();
        let redis_version: String = info.get("redis_version").unwrap();
        redis_version
    };

    let body = json!({
        "redis_version": rv
    });

    let res = create_response(
            &state,
            StatusCode::Ok,
            Some((
                body.to_string().into_bytes(),
                mime::APPLICATION_JSON
            )),
        );

    (state, res)
}