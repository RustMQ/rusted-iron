extern crate serde_json;
extern crate mime;
extern crate redis;

use hyper::{Body, StatusCode};
use futures::{future, Future, Stream};
use gotham::state::{FromState, State};
use gotham::handler::{HandlerFuture, IntoHandlerError};
use gotham::http::response::create_response;

use middleware::redis::RedisPool;

use redis::{InfoDict};

pub fn version(mut state: State) -> Box<HandlerFuture> {
    let f = Body::take_from(&mut state)
            .concat2()
            .then(|full_body| match full_body {
                Ok(_valid_body) => {
                    let connection = {
                        let redis_pool = RedisPool::borrow_mut_from(&mut state);
                        let connection = redis_pool.conn().unwrap();
                        connection
                    };

                    let rv = {
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

                    future::ok((state, res))
                },
                Err(e) => future::err((state, e.into_handler_error()))
            });

    Box::new(f)
}