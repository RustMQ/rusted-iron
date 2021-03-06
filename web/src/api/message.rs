extern crate mime;
extern crate serde_json;

use serde_json::Value;
use hyper::{Body, StatusCode};
use futures::{future, Future, Stream};
use gotham::{
    handler::{
        HandlerFuture, IntoHandlerError
    },
    http::response::create_response,
    state::{
        FromState, State
    }
};
use middleware::redis::RedisPool;
use api::queue::QueuePathExtractor;
use mq::message::{MAXIMUM_NUMBER_TO_PEEK};
use queue::message::Message;

#[derive(Deserialize, StateData, StaticResponseExtender)]
pub struct QueryStringExtractor {
    n: Option<i32>,
}

pub fn delete(mut state: State) -> Box<HandlerFuture> {
        let f = Body::take_from(&mut state)
            .concat2()
            .then(|full_body| match full_body {
                Ok(valid_body) => {
                    let connection = {
                        let redis_pool = RedisPool::borrow_mut_from(&mut state);
                        let connection = redis_pool.conn().unwrap();
                        connection
                    };

                    let (queue_name, message_id): (String, String) = {
                        let path = QueuePathExtractor::borrow_from(&state);
                        (path.name.clone().unwrap(), path.message_id.clone().unwrap())
                    };

                    let request_body :Result<Value, serde_json::Error> = serde_json::from_slice(&valid_body.to_vec());
                    let message = ::mq::message::get_message(&queue_name, &message_id, &connection);

                    let (response_message, status_code) = match &message {
                        Ok(message) => {
                            match request_body {
                                Ok(request_body) => {
                                    let reservation_id = request_body["reservation_id"].as_str().unwrap_or("");
                                    let request_message = MessageDeleteBodyRequest {
                                        id: message_id.clone(),
                                        reservation_id: Some(reservation_id.to_string()),
                                    };
                                    match request_message.is_valid_for(&message) {
                                                true => {
                                                    match ::mq::message::delete(queue_name, message_id, &connection) {
                                                        Ok(_deleted) => ("Deleted", StatusCode::Ok),
                                                        Err(_) => ("Message not found", StatusCode::NotFound)
                                                    }
                                                },
                                                false => ("A reservation_id is required", StatusCode::BadRequest),
                                            }
                                },
                                Err(_) => ("Failed to decode JSON.", StatusCode::BadRequest)       
                            }
                        },
                        Err(_) => ("Message not found", StatusCode::NotFound),
                    };

                    let body = json!({
                        "msg": response_message
                    });

                    let res = create_response(
                        &state,
                        status_code,
                        Some((
                            body.to_string().into_bytes(),
                            mime::APPLICATION_JSON
                        ))
                    );
                    return future::ok((state, res));
                },
                Err(e) => future::err((state, e.into_handler_error()))
            });

        Box::new(f)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageDeleteBodyRequest{
    pub id: String,
    pub reservation_id: Option<String>
}

impl MessageDeleteBodyRequest {
    pub fn is_valid_for(&self, message: &Message) -> bool {
        let is_reserved = match message.is_reserved() {
            Some(is_reserved) => is_reserved,
            None => false,
        };

        let is_valid = if is_reserved {
            match &self.reservation_id {
                Some(reservation_id) => {
                    reservation_id == &message.clone().reservation_id.unwrap()
                },
                None => false,
            }
        } else {
            true
        };

        is_valid
    }
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
                        path.name.clone().unwrap()
                    };

                    let body_content: Value = serde_json::from_slice(&valid_body.to_vec()).unwrap();
                    if body_content["ids"].is_null() {
                        match ::mq::message::clear_messages(&queue_name, &connection) {
                            Ok(_res) => {
                                let body = json!({
                                  "msg": "Cleared"
                                });
                                let res = create_response(
                                    &state,
                                    StatusCode::Ok,
                                    Some((
                                        body.to_string().into_bytes(),
                                        mime::APPLICATION_JSON
                                    ))
                                );

                                return future::ok((state, res));
                            },
                            Err(err) => {
                                let body = json!({
                                  "msg": err.to_string()
                                });
                                let res = create_response(
                                    &state,
                                    StatusCode::InternalServerError,
                                    Some((
                                        body.to_string().into_bytes(),
                                        mime::APPLICATION_JSON
                                    ))
                                );

                                return future::ok((state, res));
                            }
                        };
                    }

                    let messages: Vec<MessageDeleteBodyRequest> = match serde_json::from_value(body_content["ids"].clone()) {
                        Ok(messages) => messages,
                        Err(_) => {
                            let body = json!({
                              "msg": "Failed to decode JSON."
                            });
                            let res = create_response(
                                &state,
                                StatusCode::BadRequest,
                                Some((
                                    body.to_string().into_bytes(),
                                    mime::APPLICATION_JSON
                                ))
                            );
                            return future::ok((state, res));
                        }
                    };
                    
                    let queue = queue_name.clone();
                    let mut invalid_messages = messages.clone().into_iter().filter(|req_message| {
                        let message = ::mq::message::get_message(&queue, &req_message.id, &connection);
                        match &message {
                            Ok(m) => !req_message.is_valid_for(m),
                            Err(_) => false
                        }
                    });

                    match invalid_messages.next() {
                        Some(_invalid_message) => {
                            let body = json!({
                                "msg": "A reservation_id is required"
                            });

                            let res = create_response(
                                &state,
                                StatusCode::BadRequest,
                                Some((
                                    body.to_string().into_bytes(),
                                    mime::APPLICATION_JSON
                                ))
                            );

                            return future::ok((state, res));
                        },
                        None => (),
                    };

                    match ::mq::message::delete_messages(queue_name, &messages, &connection) {
                        Ok(_deleted) => {
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

                            return future::ok((state, res));
                        },
                        Err(_e) => {
                            let res = create_response(&state, StatusCode::NotFound, None);
                            return future::ok((state, res));
                        }
                    }
                },
                Err(e) => future::err((state, e.into_handler_error()))
            });

        Box::new(f)
}

pub fn get_message(mut state: State) -> Box<HandlerFuture> {
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
                        let path = QueuePathExtractor::borrow_from(&state);
                        (path.name.clone().unwrap(), path.message_id.clone().unwrap())
                    };

                    match ::mq::message::get_message(&queue_name, &message_id, &connection) {
                        Ok(msg) => {
                            let body = json!({
                                "message": msg
                            });

                            let res = create_response(
                                &state,
                                StatusCode::Ok,
                                Some((
                                    body.to_string().into_bytes(),
                                    mime::APPLICATION_JSON
                                ))
                            );

                            return future::ok((state, res));
                        },
                        Err(e) => {
                            let body = json!({
                                "msg": e.to_string()
                            });

                            let res = create_response(
                                &state,
                                StatusCode::NotFound,
                                Some((
                                    body.to_string().into_bytes(),
                                    mime::APPLICATION_JSON
                                ))
                            );

                            return future::ok((state, res));
                        }
                    }
                },
                Err(e) => future::err((state, e.into_handler_error()))
            });

        Box::new(f)
}

pub fn touch_message(mut state: State) -> Box<HandlerFuture> {
        let f = Body::take_from(&mut state)
            .concat2()
            .then(|full_body| match full_body {
                Ok(valid_body) => {
                    let connection = {
                        let redis_pool = RedisPool::borrow_mut_from(&mut state);
                        let connection = redis_pool.conn().unwrap();
                        connection
                    };

                    let (queue_name, message_id): (String, String) = {
                        let path = QueuePathExtractor::borrow_from(&state);
                        (path.name.clone().unwrap(), path.message_id.clone().unwrap())
                    };

                    let body_content: Value = serde_json::from_slice(&valid_body.to_vec()).unwrap();
                    let old_reservation_id: String = serde_json::from_value(body_content["reservation_id"].clone()).unwrap();

                    match ::mq::message::touch_message(&queue_name, &message_id, &old_reservation_id, &connection) {
                        Ok(reservation_id) => {
                            let body = json!({
                                "reservation_id": reservation_id,
                                "msg": "Touched"
                            });

                            let res = create_response(
                                &state,
                                StatusCode::Ok,
                                Some((
                                    body.to_string().into_bytes(),
                                    mime::APPLICATION_JSON
                                ))
                            );

                            return future::ok((state, res));
                        },
                        Err(_e) => {
                            let res = create_response(&state, StatusCode::NotFound, None);
                            return future::ok((state, res))
                        }
                    }
                },
                Err(e) => future::err((state, e.into_handler_error()))
            });

        Box::new(f)
}

pub fn peek_messages(mut state: State) -> Box<HandlerFuture> {
        let f = Body::take_from(&mut state)
            .concat2()
            .then(|full_body| match full_body {
                Ok(_valid_body) => {
                    let connection = {
                        let redis_pool = RedisPool::borrow_mut_from(&mut state);
                        let connection = redis_pool.conn().unwrap();
                        connection
                    };

                    let queue_name: String = {
                        let path = QueuePathExtractor::borrow_from(&state);
                        path.name.clone().unwrap()
                    };

                    let n: i32 = {
                        let path = QueryStringExtractor::borrow_from(&state);
                        match path.n {
                            Some(n) => n,
                            None => MAXIMUM_NUMBER_TO_PEEK,
                        }
                    };

                    match ::mq::message::peek_messages(&queue_name, &n, &connection) {
                        Ok(msgs) => {
                            let body = json!({
                                "messages": msgs
                            });

                            let res = create_response(
                                &state,
                                StatusCode::Ok,
                                Some((
                                    body.to_string().into_bytes(),
                                    mime::APPLICATION_JSON
                                ))
                            );

                            return future::ok((state, res));
                        },
                        Err(_e) => {
                            let res = create_response(&state, StatusCode::NotFound, None);
                            return future::ok((state, res));
                        }
                    }
                },
                Err(e) => future::err((state, e.into_handler_error()))
            });

        Box::new(f)
}


pub fn release_message(mut state: State) -> Box<HandlerFuture> {
        let f = Body::take_from(&mut state)
            .concat2()
            .then(|full_body| match full_body {
                Ok(valid_body) => {
                    let connection = {
                        let redis_pool = RedisPool::borrow_mut_from(&mut state);
                        let connection = redis_pool.conn().unwrap();
                        connection
                    };

                    let (queue_name, message_id): (String, String) = {
                        let path = QueuePathExtractor::borrow_from(&state);
                        (path.name.clone().unwrap(), path.message_id.clone().unwrap())
                    };

                    let body_content: Value = serde_json::from_slice(&valid_body.to_vec()).unwrap();
                    let reservation_id: String = serde_json::from_value(body_content["reservation_id"].clone()).unwrap();

                    let released = ::mq::message::release_message(&queue_name, &message_id, &reservation_id, &connection).unwrap();

                    if !released {
                        let res = create_response(&state, StatusCode::NotFound, None);
                        return future::ok((state, res))
                    }

                    let body = json!({
                        "msg": "Released"
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


pub fn get_push_statuses(mut state: State) -> Box<HandlerFuture> {
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
                        let path = QueuePathExtractor::borrow_from(&state);
                        (path.name.clone().unwrap(), path.message_id.clone().unwrap())
                    };

                    match ::mq::message::get_push_statuses(&queue_name, &message_id, &connection) {
                        Ok(subscribers) => {
                            let body = json!({
                                "subscribers": subscribers
                            });

                            let res = create_response(
                                &state,
                                StatusCode::Ok,
                                Some((
                                    body.to_string().into_bytes(),
                                    mime::APPLICATION_JSON
                                ))
                            );

                            return future::ok((state, res));
                        },
                        Err(e) => {
                            let body = json!({
                                "msg": e.to_string()
                            });

                            let res = create_response(
                                &state,
                                StatusCode::NotFound,
                                Some((
                                    body.to_string().into_bytes(),
                                    mime::APPLICATION_JSON
                                ))
                            );

                            return future::ok((state, res));
                        }
                    }
                },
                Err(e) => future::err((state, e.into_handler_error()))
            });

        Box::new(f)
}
