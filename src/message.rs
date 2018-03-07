extern crate redis;

use redis::*;
use queue::Queue;
use std::collections::{HashMap, HashSet};
use objectid::{ObjectId};

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<String>,
    pub body: Option<String>,
    pub reservation_id: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReserveMessageParams {
    pub n: i32
}

pub const UNRESERVED_SET_KEY: &'static str = "queue:1:unreserved:msg";
pub const RESERVED_SET_KEY: &'static str = "queue:1:reserved:msg";

impl Message {
    pub fn new() -> Message {
        Message {
            id: None,
            body: None,
            reservation_id: None
        }
    }

    pub fn new_from_hash(message_id: String, hash_map: HashMap<String, String>) -> Message {
        let mut msg = Message::new();
        msg.id = Some(message_id);

        match hash_map.get(&*"body") {
            Some(v) => {
                msg.body = Some(v.to_string());
            },
            _ => println!("Wrong key body"),
        }

        match hash_map.get(&*"reservation_id") {
            Some(v) => {
                msg.reservation_id = Some(v.to_string());
            },
            _ => println!("Wrong key for reservation id field"),
        }

        msg
    }

    pub fn push_message(queue: Queue, message: Message, con: &Connection) -> i32 {
        println!("Message: {:?}", message);
        let queue_id: String = queue.id.expect("Queue ID");
        let queue_key: String = Queue::get_queue_key(&queue_id);
        let msg_counter_key = Queue::get_message_counter_key(&queue_id);
        let mut msg_key = String::new();
        msg_key.push_str(&queue_key);
        msg_key.push_str(":msg:");
        let msg_body: String = message.body.expect("Message body");

        let msg_id = redis::transaction(con, &[&msg_counter_key], |pipe| {
            let msg_id: i32 = cmd("GET").arg(&msg_counter_key).query(con).unwrap();
            msg_key.push_str(&msg_id.to_string());
            let response: Result<Vec<String>, RedisError> = pipe
                    .atomic()
                    .cmd("HMSET")
                        .arg(&msg_key)
                        .arg("body")
                        .arg(&msg_body)
                        .arg("id")
                        .arg(&msg_id.to_string())
                    .cmd("ZADD")
                        .arg(&UNRESERVED_SET_KEY.to_string())
                        .arg(&msg_id.to_string())
                        .arg(&msg_key)
                        .ignore()
                    .cmd("INCR")
                        .arg(&msg_counter_key)
                        .ignore()
                    .cmd("HINCRBY")
                        .arg(&queue_key)
                        .arg("totalrecv")
                        .arg("1")
                        .ignore()
                    .query(con);

            match response {
                Err(e) => print!("{:?}", e),
                Ok(v) => {
                    if v[0] == String::from("OK") {
                        println!("New message set");
                    }
                    else {
                        println!("New message not set");
                    }
                    println!("V: {:?}", v);
                }
            }

            Ok(Some(msg_id))
        }).unwrap();

        msg_id
    }

    pub fn get_message(queue_id: &String, message_id: &String, con: &Connection) -> Message {
        let queue_key = Queue::get_queue_key(queue_id);
        let mut msg_key = String::new();
        msg_key.push_str(&queue_key);
        msg_key.push_str(":msg:");
        msg_key.push_str(&message_id.to_string());

        let result: HashMap<String, String> = cmd("HGETALL").arg(msg_key).query(con).unwrap();
        if result.is_empty() {
            return Message::new()
        }

        Message::new_from_hash(message_id.to_string(), result)
    }

    pub fn delete_message(queue_id: &String, message_id: &String, con: &Connection) -> bool {
        let queue_key = Queue::get_queue_key(queue_id);
        let mut key = String::new();
        key.push_str(&queue_key);
        key.push_str(":msg:");
        key.push_str(&message_id.to_string());

        let result: i32 = cmd("DEL").arg(key).query(con).unwrap();

        result >= 1
    }

    pub fn reserve_messages(queue_id: &String, reserve_params: &ReserveMessageParams, con: &Connection) -> Vec<Message> {
        let mut result = Vec::new();

        let unreserved_msg_key_list: Result<Vec<(String, isize)>, RedisError> = con.zrangebyscore_limit_withscores(&UNRESERVED_SET_KEY.to_string(), "0", "+inf", 0, reserve_params.n as isize);
        let mut message_list_for_move = Vec::new();
        let mut message_list_for_delete = Vec::new();
        match unreserved_msg_key_list {
            Ok(unreserved_msg_key_list) => {
                for msg_key in unreserved_msg_key_list {
                  message_list_for_move.push((msg_key.1.clone(), msg_key.0.clone()));
                  message_list_for_delete.push(msg_key.0.clone());
                };
            },
            Err(err) => println!("Error: {:?}", err),
        }

        if message_list_for_move.is_empty() {
            return result
        }
        // 1. TX unreserved -> reserved
        let _r: Vec<isize> = redis::transaction(con, &[UNRESERVED_SET_KEY.to_string(), RESERVED_SET_KEY.to_string()], |pipe| {
            pipe
                .atomic()
                .zrem(UNRESERVED_SET_KEY.to_string(), message_list_for_delete.clone()).ignore()
                .zadd_multiple(RESERVED_SET_KEY.to_string(), &message_list_for_move)
                .query(con)
        }).unwrap();

        // 2. Loop over reserved (per n in request)
        for rm in message_list_for_delete.clone() {
            let oid: ObjectId = ObjectId::new().unwrap();
            let _r: Vec<isize> = redis::transaction(con, &[rm.clone()], |pipe| {
        // 2.2. update msg --> TX (?)
                pipe
                    .atomic()
                    .hset_nx(rm.clone(), "reservation_id", oid.to_string())
                    .query(con)
            }).unwrap();
        };
        // 3. collect updated msgs
        for updated_msg_key in message_list_for_delete.clone() {
            let mut updated_msg: Result<HashMap<String,String>, RedisError> = con.hgetall(&updated_msg_key);
            match updated_msg {
                Ok(msg) => result.push(Message::new_from_hash(msg["id"].clone(), msg)),
                Err(err) => println!("Error: {:?}", err),
            }
        };

        result
    }
}