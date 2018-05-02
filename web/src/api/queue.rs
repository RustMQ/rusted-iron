extern crate mime;
extern crate serde_json;

use hyper::{Body, StatusCode};
use futures::{future, Future, Stream};
use gotham::{
    handler::{
        HandlerFuture, IntoHandlerError
    },
    http::response::create_response,
    state::{
        FromState,
        State
    }
};
use serde_json::Value;

use middleware::redis::RedisPool;

use mq::{
    message::{
        Message,
        ReserveMessageParams
    },
    queue::*
};
use queue::{
    queue_info::{QueueInfo, QueueSubscriber}
};

#[derive(Deserialize, StateData, StaticResponseExtender)]
pub struct QueuePathExtractor {
    pub name: String,
}

pub fn put_queue(mut state: State) -> Box<HandlerFuture> {
    let f = Body::take_from(&mut state)
        .concat2()
        .then(|full_body| match full_body {
            Ok(_valid_body) => {
                let connection = {
                    let redis_pool = RedisPool::borrow_mut_from(&mut state);
                    let connection = redis_pool.conn().unwrap();
                    connection
                };
                let name: String = {
                    let path = QueuePathExtractor::borrow_from(&state);
                    path.name.clone()
                };

                let body_content = String::from_utf8(_valid_body.to_vec()).unwrap();
                let v: Value = serde_json::from_str(&body_content).unwrap();
                let q: QueueInfo = serde_json::from_value(v["queue"].clone()).unwrap();

                let queue = create_queue2(q, &connection);

                let body = json!({
                    "queue": queue
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

pub fn push_messages(mut state: State) -> Box<HandlerFuture> {
    let f = Body::take_from(&mut state)
        .concat2()
        .then(|full_body| match full_body {
            Ok(valid_body) => {
                let ids = {
                    let connection = {
                        let redis_pool = RedisPool::borrow_mut_from(&mut state);
                        let connection = redis_pool.conn().unwrap();
                        connection
                    };
                    let name: String = {
                        let path = QueuePathExtractor::borrow_from(&state);
                        path.name.clone()
                    };

                    let mut messages: Vec<Message> = {
                        let body_content: Value = serde_json::from_slice(&valid_body.to_vec()).unwrap();
                        serde_json::from_value(body_content["messages"].clone()).unwrap()
                    };

                    let q = get_queue(&name, &connection);
                    let mut result: Vec<String> = messages
                        .into_iter()
                        .map(|msg| {
                            let mut m: Message = Message::new();
                            m.body = msg.body;
                            let mid = post_message(q.clone(), m, &*connection).expect("Message put on queue.");
                            mid.to_string()
                        }).collect();

                    result
                };

                let body = json!({
                    "ids": ids,
                    "msg": String::from("Messages put on queue.")
                });

                let res = create_response(
                    &state,
                    StatusCode::Created,
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

pub fn reserve_messages(mut state: State) -> Box<HandlerFuture> {
    let f = Body::take_from(&mut state)
        .concat2()
        .then(|full_body| match full_body {
            Ok(valid_body) => {

                let messages: Vec<Message> = {
                    let connection = {
                        let redis_pool = RedisPool::borrow_mut_from(&mut state);
                        let connection = redis_pool.conn().unwrap();
                        connection
                    };
                    let name: String = {
                        let path = QueuePathExtractor::borrow_from(&state);
                        path.name.clone()
                    };

                    let body_content = String::from_utf8(valid_body.to_vec()).unwrap();
                    let mut reserve_params: ReserveMessageParams = serde_json::from_str(&body_content).unwrap();
                    if reserve_params.delete.is_none() {
                        reserve_params.delete = Some(false)
                    }

                    Message::reserve_messages(&name, &reserve_params, &connection)
                };

                let body = json!({
                    "messages": messages
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

pub fn list_queues(mut state: State) -> Box<HandlerFuture> {
        let f = Body::take_from(&mut state)
            .concat2()
            .then(|full_body| match full_body {
                Ok(_valid_body) => {
                    let connection = {
                        let redis_pool = RedisPool::borrow_mut_from(&mut state);
                        let connection = redis_pool.conn().unwrap();
                        connection
                    };

                    let body = json!({
                        "queues": ::mq::queue::list_queues(&connection)
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

pub fn delete_queue(mut state: State) -> Box<HandlerFuture> {
        let f = Body::take_from(&mut state)
            .concat2()
            .then(|full_body| match full_body {
                Ok(_valid_body) => {
                    let connection = {
                        let redis_pool = RedisPool::borrow_mut_from(&mut state);
                        let connection = redis_pool.conn().unwrap();
                        connection
                    };

                    let name: String = {
                        let path = QueuePathExtractor::borrow_from(&state);
                        path.name.clone()
                    };

                    delete(name, &connection);

                    let body = json!({
                        "msg": "Deleted."
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

pub fn push_messages_via_webhook(mut state: State) -> Box<HandlerFuture> {
    let f = Body::take_from(&mut state)
        .concat2()
        .then(|full_body| match full_body {
            Ok(valid_body) => {
                let connection = {
                    let redis_pool = RedisPool::borrow_mut_from(&mut state);
                    let connection = redis_pool.conn().unwrap();
                    connection
                };
                let name: String = {
                    let path = QueuePathExtractor::borrow_from(&state);
                    path.name.clone()
                };
                let body_content: Value = serde_json::from_slice(&valid_body.to_vec()).unwrap();
                let mut message: Message = Message::new();
                message.body = Some(body_content.to_string());
                let q = get_queue(&name, &connection);
                let id = post_message(q, message, &*connection).expect("Message put on queue.");

                let body = json!({
                    "id": id,
                    "msg": String::from("Messages put on queue.")
                });

                let res = create_response(
                    &state,
                    StatusCode::Created,
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

pub fn get_queue_info(mut state: State) -> Box<HandlerFuture> {
        let f = Body::take_from(&mut state)
            .concat2()
            .then(|full_body| match full_body {
                Ok(_valid_body) => {
                    let connection = {
                        let redis_pool = RedisPool::borrow_mut_from(&mut state);
                        let connection = redis_pool.conn().unwrap();
                        connection
                    };

                    let name: String = {
                        let path = QueuePathExtractor::borrow_from(&state);
                        path.name.clone()
                    };

                    let queue_info;
                    match ::mq::queue::get_queue_info(name, &connection) {
                        Ok(res) => queue_info = res,
                        Err(err) => {
                            info!("Error: {:?}", err);
                            queue_info = QueueInfo::new("panic!".to_owned())
                        }
                    };

                    let body = json!({
                        "queue": queue_info
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

pub fn update_subscribers(mut state: State) -> Box<HandlerFuture> {
    let f = Body::take_from(&mut state)
        .concat2()
        .then(|full_body| match full_body {
            Ok(valid_body) => {
                let connection = {
                    let redis_pool = RedisPool::borrow_mut_from(&mut state);
                    let connection = redis_pool.conn().unwrap();
                    connection
                };
                let name: String = {
                    let path = QueuePathExtractor::borrow_from(&state);
                    path.name.clone()
                };

                let mut subscribers: Vec<QueueSubscriber> = {
                    let body_content: Value = serde_json::from_slice(&valid_body.to_vec()).unwrap();
                    serde_json::from_value(body_content["subscribers"].clone()).unwrap()
                };

                let updated = ::mq::queue::update_subscribers(name, subscribers, &connection);
                let body;
                if updated {
                    body = json!({
                        "msg": String::from("Updated")
                    });

                } else {
                    body = json!({
                        "msg": String::from("Not Updated")
                    });
                }
                let res = create_response(
                    &state,
                    StatusCode::Ok,
                    Some((
                        body.to_string().into_bytes(),
                        mime::APPLICATION_JSON
                    )),
                );

                return future::ok((state, res));
            },
            Err(e) => future::err((state, e.into_handler_error()))
        });

    Box::new(f)
}

pub fn replace_subscribers(mut state: State) -> Box<HandlerFuture> {
    let f = Body::take_from(&mut state)
        .concat2()
        .then(|full_body| match full_body {
            Ok(valid_body) => {
                let connection = {
                    let redis_pool = RedisPool::borrow_mut_from(&mut state);
                    let connection = redis_pool.conn().unwrap();
                    connection
                };
                let name: String = {
                    let path = QueuePathExtractor::borrow_from(&state);
                    path.name.clone()
                };

                let mut subscribers: Vec<QueueSubscriber> = {
                    let body_content: Value = serde_json::from_slice(&valid_body.to_vec()).unwrap();
                    serde_json::from_value(body_content["subscribers"].clone()).unwrap()
                };

                let updated = ::mq::queue::replace_subscribers(name, subscribers, &connection);
                let body;
                if updated {
                    body = json!({
                        "msg": String::from("Updated")
                    });

                } else {
                    body = json!({
                        "msg": String::from("Not Updated")
                    });
                }
                let res = create_response(
                    &state,
                    StatusCode::Ok,
                    Some((
                        body.to_string().into_bytes(),
                        mime::APPLICATION_JSON
                    )),
                );

                return future::ok((state, res));
            },
            Err(e) => future::err((state, e.into_handler_error()))
        });

    Box::new(f)
}

pub fn delete_subscribers(mut state: State) -> Box<HandlerFuture> {
    let f = Body::take_from(&mut state)
        .concat2()
        .then(|full_body| match full_body {
            Ok(valid_body) => {
                let connection = {
                    let redis_pool = RedisPool::borrow_mut_from(&mut state);
                    let connection = redis_pool.conn().unwrap();
                    connection
                };
                let name: String = {
                    let path = QueuePathExtractor::borrow_from(&state);
                    path.name.clone()
                };

                let mut subscribers: Vec<QueueSubscriber> = {
                    let body_content: Value = serde_json::from_slice(&valid_body.to_vec()).unwrap();
                    serde_json::from_value(body_content["subscribers"].clone()).unwrap()
                };

                let updated = ::mq::queue::delete_subscribers(name, subscribers, &connection);
                let body;
                if updated {
                    body = json!({
                        "msg": String::from("Updated")
                    });

                } else {
                    body = json!({
                        "msg": String::from("Not Updated")
                    });
                }
                let res = create_response(
                    &state,
                    StatusCode::Ok,
                    Some((
                        body.to_string().into_bytes(),
                        mime::APPLICATION_JSON
                    )),
                );

                return future::ok((state, res));
            },
            Err(e) => future::err((state, e.into_handler_error()))
        });

    Box::new(f)
}
