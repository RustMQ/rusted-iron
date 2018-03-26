extern crate mime;
extern crate serde_json;

use hyper::{Body, StatusCode};
use futures::{future, Future, Stream};
use gotham::state::{FromState, State};
use gotham::handler::{HandlerFuture, IntoHandlerError};
use gotham::http::response::create_response;

use middleware::redis::RedisPool;

use mq::message::Message;

#[derive(Debug, Deserialize, StateData, StaticResponseExtender)]
pub struct MessagePathExtractor {
    name: String,
    message_id: String
}

pub fn delete(mut state: State) -> Box<HandlerFuture> {
        let f = Body::take_from(&mut state)
            .concat2()
            .then(|full_body| match full_body {
                Ok(_valid_body) => {
                    let connection = {
                        let redis_pool = RedisPool::borrow_mut_from(&mut state);
                        let connection = redis_pool.conn().unwrap();
                        connection
                    };

                    info!("before msg path");
                    let (queue_name, message_id): (String, String) = {
                        let path = MessagePathExtractor::borrow_from(&state);
                        info!("path={:?}", path);
                        (path.name.clone(), path.message_id.clone())
                    };
                    info!("name = {:?}, msg_id = {:?}", queue_name, message_id);

                    Message::delete(queue_name, message_id, &connection);

                    let body = json!({
                        "msg": "Deleted"
                    });

                    let res = create_response(
                        &state,
                        StatusCode::Ok,
                        Some((
                            body.to_string().into_bytes(),
                            mime::APPLICATION_JSON
                        ))
                    );

                    future::ok((state, res))
                },
                Err(e) => future::err((state, e.into_handler_error()))
            });

        Box::new(f)
}
