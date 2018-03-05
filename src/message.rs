extern crate redis;

use redis::*;
use queue::Queue;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<String>,
    pub body: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReserveMessageParams {
    pub n: i32
}

pub const UNRESERVED_MSG_KEY_PART: &'static str = ":unreserved";
pub const RESERVED_MSG_KEY_PART: &'static str = ":reserved";

impl Message {
    pub fn new() -> Message {
        Message {
            id: None,
            body: None
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

        msg
    }

    pub fn push_message(queue: Queue, message: Message, con: &Connection) -> i32 {
        println!("Message: {:?}", message);
        let queue_id: String = queue.id.expect("Message ID");
        let queue_key: String = Queue::get_queue_key(&queue_id);
        let msg_counter_key = Queue::get_message_counter_key(&queue_id, &UNRESERVED_MSG_KEY_PART.to_string());
        let mut msg_key = String::new();
        msg_key.push_str(&queue_key);
        msg_key.push_str(":msg:");
        let msg_body: String = message.body.expect("Message body");

        let msg_id = redis::transaction(con, &[&msg_counter_key, &queue_key], |pipe| {
            let msg_id: i32 = cmd("GET").arg(&msg_counter_key).query(con).unwrap();
            msg_key.push_str(&msg_id.to_string());
            let response: Result<Vec<i32>, RedisError> = pipe
                    .cmd("HSET")
                        .arg(&msg_key)
                        .arg("body")
                        .arg(&msg_body)
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
                    if v[0] == 1 {
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
}