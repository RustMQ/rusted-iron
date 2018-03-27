extern crate redis;

use api::message::MessageDeleteBodyRequest;
use redis::*;
use mq::queue::Queue;
use std::collections::{HashMap};
use objectid::{ObjectId};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub id: Option<String>,
    pub body: Option<String>,
    pub reservation_id: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReserveMessageParams {
    pub n: i32,
    pub delete: Option<bool>
}

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
        let queue_name: String = queue.name.expect("Queue Name");
        let queue_key: String = Queue::get_queue_key(&queue_name);

        let mut queue_unreserved_key = String::new();
        queue_unreserved_key.push_str(&queue_key.clone());
        queue_unreserved_key.push_str(":unreserved:msg");

        let msg_counter_key = Queue::get_message_counter_key(&queue_name);
        let mut msg_key = String::new();
        msg_key.push_str(&queue_key);
        msg_key.push_str(":msg:");
        let msg_body: String = message.body.expect("Message body");

        let msg_id = redis::transaction(con, &[&msg_counter_key], |pipe| {
            let msg_id: i32 = cmd("GET").arg(&msg_counter_key).query(con).unwrap();
            msg_key.push_str(&msg_id.to_string());
            let _response: Result<Vec<String>, RedisError> = pipe
                    .atomic()
                    .cmd("HMSET")
                        .arg(&msg_key)
                        .arg("body")
                        .arg(&msg_body)
                        .arg("id")
                        .arg(&msg_id.to_string())
                    .cmd("ZADD")
                        .arg(&queue_unreserved_key)
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

    pub fn delete_message(queue_name: &String, message: &Message, con: &Connection) -> bool {
        let mut queue_key = String::new();
        queue_key.push_str("queue:");
        queue_key.push_str(&queue_name);

        let mut queue_unreserved_key = String::new();
        queue_unreserved_key.push_str(&queue_key.clone());
        queue_unreserved_key.push_str(":unreserved:msg");

        let mut queue_reserved_key = String::new();
        queue_reserved_key.push_str(&queue_key.clone());
        queue_reserved_key.push_str(":reserved:msg");

        let mut msg_key = String::new();
        msg_key.push_str(&queue_key);
        msg_key.push_str(":msg:");
        msg_key.push_str(&message.id.clone().unwrap());

        let _ : Vec<bool> = pipe()
            .zrem(&queue_reserved_key, &[&msg_key]).ignore()
            .zrem(&queue_unreserved_key, &[&msg_key]).ignore()
            .del(&msg_key)
            .query(con).unwrap();

        true
    }

    pub fn reserve_messages(queue_name: &String, reserve_params: &ReserveMessageParams, con: &Connection) -> Vec<Message> {
        let mut result = Vec::new();

        let mut queue_key = String::new();
        queue_key.push_str("queue:");
        queue_key.push_str(&queue_name);

        let mut queue_unreserved_key = String::new();
        queue_unreserved_key.push_str(&queue_key.clone());
        queue_unreserved_key.push_str(":unreserved:msg");

        let mut queue_reserved_key = String::new();
        queue_reserved_key.push_str(&queue_key.clone());
        queue_reserved_key.push_str(":reserved:msg");

        let unreserved_msg_key_list: Result<Vec<(String, isize)>, RedisError> = con.zrangebyscore_limit_withscores(&queue_unreserved_key, "0", "+inf", 0, reserve_params.n as isize);
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
        let _r: Vec<isize> = redis::transaction(con, &[&queue_unreserved_key, &queue_reserved_key], |pipe| {
            pipe
                .atomic()
                .zrem(&queue_unreserved_key.clone(), message_list_for_delete.clone()).ignore()
                .zadd_multiple(&queue_reserved_key.clone(), &message_list_for_move)
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

        if reserve_params.delete == Some(true) {
            for message in result.clone() {
                Message::delete_message(&queue_name, &message, con);
            }
        }

        result
    }

    pub fn delete(queue_name: String, message_id: String, con: &Connection) -> bool {
        let m = Message {
            id: Some(message_id),
            body: None,
            reservation_id: None
        };

        Message::delete_message(&queue_name, &m, con)
    }

    pub fn delete_messages(queue_name: String, messages: &Vec<MessageDeleteBodyRequest>, con: &Connection) -> Vec<bool> {
        let mut res = Vec::new();

        for m in messages {
            res.push(Message::delete(queue_name.to_string(), m.id.to_owned(), con));
        }

        res
    }
}