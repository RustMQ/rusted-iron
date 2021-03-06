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
        ReserveMessageParams
    },
    queue::{create_queue, delete, get_queue, post_message, patch_queue_info}
};
use queue::{
    queue_info::{QueueInfo, QueueSubscriber, QueueState},
    message::*
};

#[derive(Debug, Deserialize, StateData, StaticResponseExtender)]
pub struct QueuePathExtractor {
    pub project_id: String,
    pub name: Option<String>,
    pub message_id: Option<String>
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
                let (project_id, name) = {
                    let path = QueuePathExtractor::borrow_from(&state);
                    (path.project_id.clone(), path.name.clone().unwrap())
                };

                let body_content = String::from_utf8(_valid_body.to_vec()).unwrap();
                let v: Value = serde_json::from_str(&body_content).unwrap();
                let mut q: QueueInfo;
                if v["queue"].is_null() {
                    q = QueueInfo::default(name);
                } else {
                    q = serde_json::from_value(v["queue"].clone()).unwrap();
                    q.name = Some(name);
                    q.fill_missed_fields();
                }

                let (body, status_code): (Value, StatusCode) = match q.state() {
                    QueueState::Valid => {
                        let mut queue = create_queue(q, &connection);
                        queue.project_id = Some(project_id);
                        let body = json!({
                            "queue": queue
                        });
                        (body, StatusCode::Ok)
                    },
                    QueueState::TypeError => {
                        let body = json!({
	                        "msg": "Queue type cannot be changed"
                        });
                        (body, StatusCode::Forbidden)
                    },
                    QueueState::SubscriberError => {
                        let body = json!({
	                        "msg": "Push queues must have at least one subscriber"
                        });
                        (body, StatusCode::BadRequest)
                    }
                };

                let res = create_response(
                    &state,
                    status_code,
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
                        path.name.clone().unwrap()
                    };

                    let mut messages: Vec<Message> = {
                        let body_content: Value = serde_json::from_slice(&valid_body.to_vec()).unwrap();
                        serde_json::from_value(body_content["messages"].clone()).unwrap()
                    };

                    let result: Vec<String> = match get_queue(&name, &connection) {
                        Ok(q) => {
                            messages
                                .into_iter()
                                .map(|msg| {
                                    let m = Message::with_body(&msg.body);
                                    let mid = post_message(q.name.clone().unwrap(), m, &*connection).unwrap();
                                    mid
                                }).collect()
                        },
                        Err(err) => {
                            debug!("Error: {:#?}", err);
                            Vec::new()
                        }
                    };

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
                let connection = {
                    let redis_pool = RedisPool::borrow_mut_from(&mut state);
                    let connection = redis_pool.conn().unwrap();
                    connection
                };
                let name: String = {
                    let path = QueuePathExtractor::borrow_from(&state);
                    path.name.clone().unwrap()
                };

                let body_content = String::from_utf8(valid_body.to_vec()).unwrap();
                let mut reserve_params: ReserveMessageParams = serde_json::from_str(&body_content).unwrap();
                if reserve_params.delete.is_none() {
                    reserve_params.delete = Some(false)
                }

                match ::mq::message::reserve_messages(&name, &reserve_params, &connection) {
                    Ok(messages) => {
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

                    match ::mq::queue::list_queues(&connection) {
                        Ok(queues) => {
                            let body = json!({
                                "queues": queues
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
                        path.name.clone().unwrap()
                    };

                    match delete(name, &connection) {
                        Ok(deleted) => {
                            let (message, status_code) = match deleted {
                                true => ("Deleted.", StatusCode::Ok),
                                false => ("Queue not found", StatusCode::NotFound),
                            };

                            let body = json!({
                                "msg": message
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
                    path.name.clone().unwrap()
                };
                let body_content: Value = serde_json::from_slice(&valid_body.to_vec()).unwrap();
                let message = Message::with_body(&body_content.to_string());

                let q = get_queue(&name, &connection).unwrap();
                let id = post_message(q.name.clone().unwrap(), message, &*connection).expect("Message put on queue.");

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
                    path.name.clone().unwrap()
                };

                let (body, status_code) = match ::mq::queue::get_queue_info(name, &connection) {
                    Ok(queue_info) => {
                        let body = json!({
                            "queue": queue_info
                        });

                        (body, StatusCode::Ok)
                    }
                    Err(_) => {
                        let body = json!({
                            "msg": "Queue not found"
                        });

                        (body, StatusCode::NotFound)
                    }
                };

                let res = create_response(
                    &state,
                    status_code,
                    Some((
                        body.to_string().into_bytes(),
                        mime::APPLICATION_JSON
                    )),
                );

                future::ok((state, res))
            }
            Err(e) => future::err((state, e.into_handler_error())),
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
                    path.name.clone().unwrap()
                };

                let mut subscribers: Vec<QueueSubscriber> = {
                    let body_content: Value = serde_json::from_slice(&valid_body.to_vec()).unwrap();
                    serde_json::from_value(body_content["subscribers"].clone()).unwrap()
                };

                let body = match ::mq::queue::update_subscribers(name, subscribers, &connection) {
                    Ok(updated) => {
                        if updated {
                            json!({
                                "msg": String::from("Updated")
                            })
                        } else {
                            json!({
                                "msg": String::from("Not Updated")
                            })
                        }
                    },
                    Err(_e) => {
                        let res = create_response(&state, StatusCode::NotFound, None);
                        return future::ok((state, res));
                    }
                };

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
                    path.name.clone().unwrap()
                };

                let mut subscribers: Vec<QueueSubscriber> = {
                    let body_content: Value = match serde_json::from_slice(&valid_body.to_vec()) {
                        Ok(body_content) => body_content,
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
                                )),
                            );

                            return future::ok((state, res));
                        }
                    };

                    match serde_json::from_value(body_content["subscribers"].clone()) {
                        Ok(subscribers) => subscribers,
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
                                )),
                            );

                            return future::ok((state, res));
                        }
                    }
                };

                if subscribers.is_empty() {
                    let body = json!({
                        "msg": "Push queues must have at least one subscriber"
                    });
                    let res = create_response(
                        &state,
                        StatusCode::BadRequest,
                        Some((
                            body.to_string().into_bytes(),
                            mime::APPLICATION_JSON
                        )),
                    );

                    return future::ok((state, res));
                };

                let body = match ::mq::queue::replace_subscribers(name, subscribers, &connection) {
                    Ok(updated) => {
                        if updated {
                            json!({
                               "msg": String::from("Updated")
                            })
                        } else {
                            json!({
                                "msg": String::from("Not Updated")
                            })
                        }
                    }
                    Err(_e) => {
                        let res = create_response(&state, StatusCode::NotFound, None);
                        return future::ok((state, res));
                    }
                };

                let res = create_response(
                    &state,
                    StatusCode::Ok,
                    Some((
                        body.to_string().into_bytes(),
                        mime::APPLICATION_JSON
                    )),
                );

                return future::ok((state, res));
            }
            Err(e) => future::err((state, e.into_handler_error())),
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
                    path.name.clone().unwrap()
                };

                let mut subscribers: Vec<QueueSubscriber> = {
                    let body_content: Value = serde_json::from_slice(&valid_body.to_vec()).unwrap();
                    serde_json::from_value(body_content["subscribers"].clone()).unwrap()
                };

                let (status_code, body) = match ::mq::queue::delete_subscribers(name, subscribers, &connection) {
                    Ok(updated) => {
                        if updated {
                            let body = json!({
                                "msg": String::from("Updated")
                            });
                            (StatusCode::Ok, body)
                        } else {
                            let body = json!({
                                "msg": String::from("Push queues must have at least one subscriber")
                            });
                            (StatusCode::BadRequest, body)
                        }
                    },
                    Err(_e) => {
                        let res = create_response(&state, StatusCode::NotFound, None);
                        return future::ok((state, res));
                    }
                };

                let res = create_response(
                    &state,
                    status_code,
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

pub fn update_queue(mut state: State) -> Box<HandlerFuture> {
    let f = Body::take_from(&mut state)
        .concat2()
        .then(|full_body| match full_body {
            Ok(_valid_body) => {
                let connection = {
                    let redis_pool = RedisPool::borrow_mut_from(&mut state);
                    let connection = redis_pool.conn().unwrap();
                    connection
                };
                let (project_id, name) = {
                    let path = QueuePathExtractor::borrow_from(&state);
                    (path.project_id.clone(), path.name.clone().unwrap())
                };

                let current_queue_info = ::mq::queue::get_queue_info(name.clone(), &connection);
                if current_queue_info.is_err() {
                    let body = json!({
                        "msg": "Queue not found"
                    });

                    let res = create_response(
                        &state,
                        StatusCode::NotFound,
                        Some((
                            body.to_string().into_bytes(),
                            mime::APPLICATION_JSON
                        )),
                    );

                    return future::ok((state, res))
                }

                let body_content = String::from_utf8(_valid_body.to_vec()).unwrap();
                let v: Value = serde_json::from_str(&body_content).unwrap();

                if v["queue"].is_null() {
                    let mut res_q = current_queue_info.unwrap().clone();
                    res_q.project_id = Some(project_id);

                    let body = json!({
                        "queue": res_q
                    });

                    let res = create_response(
                        &state,
                        StatusCode::Ok,
                        Some((
                            body.to_string().into_bytes(),
                            mime::APPLICATION_JSON
                        )),
                    );

                    return future::ok((state, res))
                }

                let new_queue_info: QueueInfo = serde_json::from_value(v["queue"].clone()).unwrap();

                let updated_queue_info_res = patch_queue_info(name.clone(), new_queue_info, &connection);
                if updated_queue_info_res.is_ok() {
                    let mut updated_queue_info = updated_queue_info_res.unwrap();
                    updated_queue_info.project_id = Some(project_id);

                    let body = json!({
                        "queue": updated_queue_info
                    });

                    let res = create_response(
                        &state,
                        StatusCode::Ok,
                        Some((
                            body.to_string().into_bytes(),
                            mime::APPLICATION_JSON
                        )),
                    );

                    return future::ok((state, res))
                }

                let err = updated_queue_info_res.unwrap_err();
                let error_message = err.to_string();

                let body = json!({
                    "msg": error_message
                });

                let status_code = match error_message.contains("subscriber") {
                    true => StatusCode::BadRequest,
                    false => StatusCode::Forbidden
                };

                let res = create_response(
                    &state,
                    status_code,
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
