extern crate mime;
extern crate serde_json;

use serde_json::Value;
use hyper::{Body, StatusCode};
use futures::{future, Future, Stream};
use gotham::state::{FromState, State};
use gotham::handler::{HandlerFuture, IntoHandlerError};
use gotham::http::response::create_response;

use middleware::redis::RedisPool;

use api::queue::QueuePathExtractor;
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

                    let (queue_name, message_id): (String, String) = {
                        let path = MessagePathExtractor::borrow_from(&state);
                        (path.name.clone(), path.message_id.clone())
                    };

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageDeleteBodyRequest{
    pub id: String
}

pub fn delete_messages(mut state: State) -> Box<HandlerFuture> {
        let f = Body::take_from(&mut state)
            .concat2()
            .then(|full_body| match full_body {
                Ok(valid_body) => {
                    let connection = {
                        let redis_pool = RedisPool::borrow_mut_from(&mut state);
                        let connection = redis_pool.conn().unwrap();
                        connection
                    };

                    let queue_name = {
                        let path = QueuePathExtractor::borrow_from(&state);
                        path.name.clone()
                    };

                    let body_content: Value = serde_json::from_slice(&valid_body.to_vec()).unwrap();
                    let messages: Vec<MessageDeleteBodyRequest> = serde_json::from_value(body_content["ids"].clone()).unwrap();

                    Message::delete_messages(queue_name, &messages, &connection);

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
