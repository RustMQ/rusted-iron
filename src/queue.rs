extern crate mime;
extern crate serde_json;

use std::collections::HashMap;
use message::{Message, ReserveMessageParams};
use redis::*;

use hyper::{Body, StatusCode};
use futures::{future, Future, Stream};
use gotham::state::{FromState, State};
use gotham::handler::{HandlerFuture, IntoHandlerError};
use gotham::http::response::create_response;

use std::sync::Arc;
use std::sync::Mutex;
use rayon::prelude::*;

use redis_middleware::RedisPool;

#[derive(Deserialize, StateData, StaticResponseExtender)]
pub struct QueuePathExtractor {
    name: String,
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
pub struct MessagePathExtractor {
    id: String,
    queue_id: String
}

#[derive(PartialEq, Eq, Clone, Debug, Copy)]
pub enum QueueError {
    CounterKeyMissing
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Queue {
    pub name: Option<String>,
    pub class: Option<String>,
    pub totalrecv: Option<i32>,
    pub totalsent: Option<i32>
}

pub const DEFAULT_QUEUE_KEY: &'static str = "queue:";

impl Queue {
    fn new() -> Queue {
        Queue {
            class: None,
            name: None,
            totalrecv: None,
            totalsent: None
        }
    }

    pub fn get_queue_key(queue_id: &String) -> String {
        let mut key = String::new();
        key.push_str(DEFAULT_QUEUE_KEY);
        key.push_str(&queue_id);

        key
    }

    fn new_from_hash(queue_name: String, hash_map: HashMap<String, String>) -> Queue {
        let mut queue = Queue::new();
        queue.name = Some(queue_name);

        match hash_map.get(&*"class") {
            Some(v) => {
                queue.class = Some(v.to_string());
            },
            _ => println!("Wrong key class"),
        }
        match hash_map.get(&*"totalrecv") {
            Some(v) => {
                queue.totalrecv = Some(v.parse::<i32>().unwrap());
            },
            _ => println!("Wrong key totalrecv"),
        }
        match hash_map.get(&*"totalsent") {
            Some(v) => {
                queue.totalsent = Some(v.parse::<i32>().unwrap());
            },
            _ => println!("Wrong key totalsent"),
        }

        queue
    }

    pub fn get_queue(queue_name: &String, con: &Connection) -> Queue {
        let queue_key = Queue::get_queue_key(queue_name);
        let result: HashMap<String, String> = cmd("HGETALL").arg(queue_key).query(con).unwrap();

        Queue::new_from_hash(queue_name.to_string(), result)
    }

    pub fn get_message_counter_key(queue_id: &String) -> String {
        let mut key = String::new();
        let queue_key = Queue::get_queue_key(queue_id);
        key.push_str(&queue_key);
        key.push_str(":msg:counter");

        key
    }

    pub fn post_message(queue: Queue, message: Message, con: &Connection) -> Result<i32, QueueError> {
        let q = Queue {
            class: queue.class,
            name: queue.name,
            totalrecv: queue.totalrecv,
            totalsent: queue.totalsent
        };
        Ok(Message::push_message(q, message, con))
    }

    pub fn get_message(queue_id: &String, message_id: &String, con: &Connection) -> Result<Message, QueueError> {
        Ok(Message::get_message(queue_id, message_id, con))
    }

    pub fn delete_message(queue_id: &String, message_id: &String, con: &Connection) -> bool {
        Message::delete_message(queue_id, message_id, con)
    }

    pub fn reserve_messages(queue_id: &String, reserve_params: &ReserveMessageParams, con: &Connection) -> Vec<Message> {
        Message::reserve_messages(queue_id, reserve_params, con)
    }

    pub fn create_queue(queue_name: String, con: &Connection) -> Queue {
        let mut queue_key = String::new();
        queue_key.push_str("queue:");
        queue_key.push_str(&queue_name);
        let mut pipe = pipe();
        pipe.cmd("SADD").arg("queues".to_string()).arg(&queue_name).ignore();
        pipe.cmd("HMSET").arg(&queue_key)
            .arg("name".to_string())
            .arg(&queue_name)
            .arg("class".to_string())
            .arg("pull".to_string())
            .arg("totalrecv".to_string())
            .arg(0)
            .arg("totalsent".to_string())
            .arg(0).ignore();
        queue_key.push_str(":msg:counter");
        let _: Vec<String> = pipe.cmd("SET").arg(queue_key).arg(0).query(con).unwrap();

        Queue {
            name: Some(queue_name),
            class: Some("pull".to_string()),
            totalrecv: Some(0),
            totalsent: Some(0)
        }
    }
}

pub fn put_queue(mut state: State) -> Box<HandlerFuture> {
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
                let queue = Queue::create_queue(name, &connection);

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
                    let body_content = String::from_utf8(valid_body.to_vec()).unwrap();
                    let messages: Vec<Message> = serde_json::from_str(&body_content).unwrap();

                    let q = Queue::get_queue(&name, &connection);
                    let mut result: Vec<String> = messages
                        .into_iter()
                        .map(|msg| {
                            let mut m: Message = Message::new();
                            m.body = msg.body;
                            let mid = Queue::post_message(q.clone(), m, &*connection).expect("Message put on queue.");
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
                    let reserve_params: ReserveMessageParams = serde_json::from_str(&body_content).unwrap();

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

#[derive(Serialize, Deserialize)]
struct QueueLite {
    name: String
}

pub fn list_queues(mut state: State) -> Box<HandlerFuture> {
        let f = Body::take_from(&mut state)
        .concat2()
        .then(|full_body| match full_body {
            Ok(valid_body) => {
                let connection = {
                        let redis_pool = RedisPool::borrow_mut_from(&mut state);
                        let connection = redis_pool.conn().unwrap();
                        connection
                    };
                let r: Vec<String> = cmd("SMEMBERS").arg("queues".to_string()).query(&*connection).unwrap();
                let mut res: Vec<QueueLite> = Vec::new();
                for queue_name in r {
                    res.push(QueueLite {
                        name: queue_name
                    })
                }
                let body = json!({
                    "queues": res
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
