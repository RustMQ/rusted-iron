extern crate redis;
extern crate serde_json;

use std::collections::{HashMap};
use objectid::{ObjectId};
use redis::*;
use api::message::MessageDeleteBodyRequest;
use mq::queue::*;
use queue::{
    message::PushMessage,
    queue::Queue,
    queue_info::{QueueInfo, QueueType}
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub id: Option<String>,
    pub body: Option<String>,
    pub reservation_id: Option<String>,
    pub source_msg_id: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReserveMessageParams {
    pub n: i32,
    pub delete: Option<bool>
}

pub const MAXIMUM_NUMBER_TO_PEEK: i32 = 1;

impl Message {
    pub fn new() -> Message {
        Message {
            id: None,
            body: None,
            reservation_id: None,
            source_msg_id: None
        }
    }

    pub fn new_from_hash(message_id: String, hash_map: HashMap<String, String>) -> Message {
        let mut msg = Message::new();
        msg.id = Some(message_id);

        match hash_map.get(&*"body") {
            Some(v) => {
                msg.body = Some(v.to_string());
            },
            _ => msg.body = None
        }

        match hash_map.get(&*"reservation_id") {
            Some(v) => {
                msg.reservation_id = Some(v.to_string());
            },
            _ => msg.reservation_id = None
        }

        match hash_map.get(&*"source_msg_id") {
            Some(v) => {
                msg.source_msg_id = Some(v.to_string());
            },
            _ => msg.source_msg_id = None
        }

        msg
    }

    pub fn push_message(queue: Queue, message: Message, con: &Connection) -> i32 {
        let queue_name: String = queue.name.expect("Queue Name");
        let queue_key: String = Queue::get_queue_key(&queue_name);

        let mut queue_unreserved_key = String::new();
        queue_unreserved_key.push_str(&queue_key.clone());
        queue_unreserved_key.push_str(":unreserved:msg");

        let msg_counter_key = get_message_counter_key(&queue_name);
        let mut msg_key = String::new();
        msg_key.push_str(&queue_key);
        msg_key.push_str(":msg:");
        let msg_body: String = message.body.expect("Message body");
        let mut msg = ::queue::message::Message {
            id: None,
            delay: None,
            body: msg_body.clone(),
            source_msg_id: None,
            reservation_id: None,
            reserved_count: None,
        };
        let msg_id = redis::transaction(con, &[&msg_counter_key], |pipe| {
            let msg_id: i32 = cmd("GET").arg(&msg_counter_key).query(con).unwrap();
            msg_key.push_str(&msg_id.to_string());
            let oid = ObjectId::new().unwrap();
            msg.source_msg_id = Some(oid.clone().to_string());
            let _response: Result<Vec<String>, RedisError> = pipe
                    .atomic()
                    .cmd("HMSET")
                        .arg(&msg_key)
                        .arg("body")
                        .arg(&msg_body)
                        .arg("id")
                        .arg(&msg_id.to_string())
                        .arg("source_msg_id")
                        .arg(&oid.to_string())
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
        msg.id = Some(msg_id.clone().to_string());
        let qi_as_string = match queue.value {
            Some(v) => v,
            None => {
                debug!("QI parse error: {:?}", queue.value);
                String::new()
            }
        };

        let qi: QueueInfo = serde_json::from_str(qi_as_string.as_str()).unwrap();
        let queue_type = qi.queue_type.clone().unwrap();
        if queue_type == QueueType::Unicast || queue_type == QueueType::Multicast {
            let pm: PushMessage = PushMessage {
                queue_info: qi,
                msg: msg
            };
            let mut msg_channel_key = String::new();
            msg_channel_key.push_str(&queue_key.clone());
            msg_channel_key.push_str(":msg:channel");

            let _ : isize = cmd("PUBLISH")
                .arg(&msg_channel_key)
                .arg(serde_json::to_string(&pm).unwrap())
                .query(con).unwrap();
        }

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
            reservation_id: None,
            source_msg_id: None,
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

    pub fn touch_message(queue_id: &String, message_id: &String, reservation_id: &String, con: &Connection) -> String {
        let queue_key = Queue::get_queue_key(queue_id);
        let mut msg_key = String::new();
        msg_key.push_str(&queue_key);
        msg_key.push_str(":msg:");
        msg_key.push_str(&message_id.to_string());

        let msg = Message::get_message(&queue_id, &message_id, con);

        let current_reservation_id = match msg.reservation_id {
            Some(reservation_id) => reservation_id,
            None => String::new(),
        };

        if current_reservation_id != reservation_id.to_string() {
            return String::new()
        }

        let oid: ObjectId = ObjectId::new().unwrap();
        let _: isize = cmd("HSET").arg(msg_key).arg("reservation_id").arg(oid.to_string()).query(con).unwrap();

        oid.to_string()
    }

    pub fn peek_messages(queue_name: &String, number_to_peek: &i32, con: &Connection) -> Vec<Message> {
        let mut result = Vec::new();

        let mut queue_key = String::new();
        queue_key.push_str("queue:");
        queue_key.push_str(&queue_name);

        let mut queue_unreserved_key = String::new();
        queue_unreserved_key.push_str(&queue_key.clone());
        queue_unreserved_key.push_str(":unreserved:msg");

        let unreserved_msg_key_list: Vec<(String, isize)> = con.zrangebyscore_limit_withscores(&queue_unreserved_key, "0", "+inf", 0, *number_to_peek as isize).unwrap();
        let mut message_key_list = Vec::new();
        for msg_key in unreserved_msg_key_list {
            message_key_list.push(msg_key.0.clone());
        };

        for msg_key in message_key_list {
            let mut msg_res: Result<HashMap<String,String>, RedisError> = con.hgetall(&msg_key);
            match msg_res {
                Ok(map) => result.push(Message::new_from_hash(map["id"].clone(), map)),
                Err(err) => println!("Error: {:?}", err),
            }
        };

        result
    }

    pub fn release_message(queue_name: &String, message_id: &String, reservation_id: &String, con: &Connection) -> bool {
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
        msg_key.push_str(&message_id.to_string());

        let msg = Message::get_message(&queue_name, &message_id, con);

        let current_reservation_id = match msg.reservation_id {
            Some(reservation_id) => reservation_id,
            None => String::new(),
        };
        if current_reservation_id != reservation_id.to_string() {
            return false
        }

        let msg_score: i32 = cmd("ZSCORE").arg(&queue_reserved_key).arg(&msg_key).query(con).unwrap();

        let mut pipe = pipe();

        pipe.zrem(&queue_reserved_key, &msg_key).ignore();
        pipe.zadd(&queue_unreserved_key, &msg_key, msg_score).ignore();
        pipe.hdel(&msg_key, "reservation_id");

        let (res,): (i32,) = pipe.query(con).unwrap();

        if res == 1 {
            return true
        } else {
            return false
        }
    }
}