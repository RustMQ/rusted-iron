extern crate mime;
extern crate serde_json;

use std::collections::HashMap;
use message::{Message, ReserveMessageParams};
use redis::*;

use hyper::{Body, Response, StatusCode};
use futures::{future, Future, Stream};
use gotham::state::{FromState, State};
use gotham::handler::{IntoResponse, HandlerFuture, IntoHandlerError};
use gotham::http::response::create_response;

use redis_middleware::RedisMiddlewareData;

#[derive(Deserialize, StateData, StaticResponseExtender)]
pub struct QueuePathExtractor {
    id: String,
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
    pub id: Option<String>,
    pub name: Option<String>,
    pub class: Option<String>,
    pub totalrecv: Option<i32>,
    pub totalsent: Option<i32>
}

pub const DEFAULT_QUEUE_KEY: &'static str = "queue:";

impl Queue {
    fn new() -> Queue {
        Queue {
            id: None,
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

    fn new_from_hash(queue_id: String, hash_map: HashMap<String, String>) -> Queue {
        let mut queue = Queue::new();
        queue.id = Some(queue_id);

        match hash_map.get(&*"name") {
            Some(v) => {
                queue.name = Some(v.to_string());
            },
            _ => println!("Wrong key name"),
        }
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

    pub fn get_queue(queue_id: &String, con: &Connection) -> Queue {
        let mut q = Queue::new();
        q.id = Some(queue_id.to_string());
        let queue_key = Queue::get_queue_key(queue_id);
        let result: HashMap<String, String> = cmd("HGETALL").arg(queue_key).query(con).unwrap();

        Queue::new_from_hash(queue_id.to_string(), result)
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
            id: queue.id,
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
}

pub fn push_messages(mut state: State) -> Box<HandlerFuture> {
    let f = Body::take_from(&mut state)
        .concat2()
        .then(|full_body| match full_body {
            Ok(valid_body) => {
                let ids = {
                    let path = QueuePathExtractor::borrow_from(&state);
                    let data = RedisMiddlewareData::borrow_from(&state);
                    let connection = &*(data.connection.0);
                    let id = path.id.parse().unwrap();
                    info!("ID: {}", id);
                    let body_content = String::from_utf8(valid_body.to_vec()).unwrap();
                    info!("BODY: {}", body_content);
                    let messages: Vec<Message> = serde_json::from_str(&body_content).unwrap();

                    let q = Queue::get_queue(&id, connection);
                    info!("Q: {:?}", q);
                    let mut result = Vec::new();
                    for x in messages {
                        let mut m: Message = Message::new();
                        m.body = x.body;
                        let mid = Queue::post_message(q.clone(), m, &*connection).expect("Message put on queue.");
                        result.push(mid.to_string());
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

                let messages: Vec<Message> = {
                    let path = QueuePathExtractor::borrow_from(&state);
                    let data = RedisMiddlewareData::borrow_from(&state);
                    let connection = &*(data.connection.0);
                    let id: String = path.id.parse().unwrap();
                    info!("ID: {}", id);
                    let body_content = String::from_utf8(valid_body.to_vec()).unwrap();
                    info!("BODY: {}", body_content);
                    let reserve_params: ReserveMessageParams = serde_json::from_str(&body_content).unwrap();

                    Message::reserve_messages(&id, &reserve_params, connection)
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
