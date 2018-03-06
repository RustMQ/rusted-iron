extern crate redis;

use redis::*;
use queue::Queue;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<String>,
    pub body: Option<String>,
    pub reserved_key: Option<String>
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
            body: None,
            reserved_key: None
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

        match hash_map.get(&*"reserved_key") {
            Some(v) => {
                msg.reserved_key = Some(v.to_string());
            },
            _ => println!("Wrong key for reserved key field"),
        }

        msg
    }

    pub fn push_message(queue: Queue, message: Message, con: &Connection) -> i32 {
        println!("Message: {:?}", message);
        let queue_id: String = queue.id.expect("Queue ID");
        let queue_key: String = Queue::get_queue_key(&queue_id);
        let msg_counter_key = Queue::get_message_counter_key(&queue_id, &UNRESERVED_MSG_KEY_PART.to_string());
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
                        .arg("queue:1:unreserved:msg")
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

        let queue_key = Queue::get_queue_key(queue_id);
        let mut unreserved_msg_key = String::new();
        unreserved_msg_key.push_str(&queue_key);
        unreserved_msg_key.push_str(&UNRESERVED_MSG_KEY_PART.to_string());
        unreserved_msg_key.push_str(":msg");
        let reserved_msg_counter_key = Queue::get_message_counter_key(&queue_id, &RESERVED_MSG_KEY_PART.to_string());

        let r = redis::transaction(con, &[&reserved_msg_counter_key], |pipe| {
            let mut score: i32 = try!(con.get(&reserved_msg_counter_key));
            println!("Score: {:?}", score);
            println!("MK: {:?}", unreserved_msg_key);

            let resp: Vec<String> = try!(con
                    .zrangebyscore_limit(&unreserved_msg_key, "0", "+inf", 0, reserve_params.n as isize));
            println!("R: {:?}", resp);

            let mut sm = Vec::new();
            let mut mfd = Vec::new();
            let mut mfr = Vec::new();

            for m_k in resp {
                println!("FOR!!!");
                let mut m: HashMap<String,String> = try!(con.hgetall(&m_k));
                println!("M: {:?}", m);
                mfd.push(m_k.clone());
                sm.push(
                    (score, m_k)
                );
                let _ : () = try!(
                    pipe.atomic().cmd("INCR")
                        .arg(&reserved_msg_counter_key,)
                        .query(con)
                );
                m.insert("reserved_key".to_string(), score.to_string());
                mfr.push(m);
                score = score + 1;
            };
            let _ : () = try!(con
                .zadd_multiple("queue:1:reserved:msg", &sm)
            );
            let _: () = try!(con.zrem("queue:1:unreserved:msg", mfd));
            // println!("SM: {:?}", sm);
            // let resp2: Value = try!(pipe.zadd_multiple("queue:1:reserved:msg", &sm).query(con));
            // println!("R: {:?}", resp2);
            for mr in mfr {
                let id: String = mr.get("id").unwrap().to_string();
                result.push(Message::new_from_hash(id, mr));
            }

            Ok(Some(1))
        }).unwrap();

        result
    }
}