extern crate serde_json;

use std::collections::HashMap;
use chrono::prelude::*;
use redis::*;

use mq::message::Message;
use queue::queue_info::QueueInfo;


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

#[derive(Serialize, Deserialize)]
pub struct QueueLite {
    name: String
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
            _ => queue.class = None
        }
        match hash_map.get(&*"totalrecv") {
            Some(v) => {
                queue.totalrecv = Some(v.parse::<i32>().unwrap());
            },
            _ => queue.totalrecv = None
        }
        match hash_map.get(&*"totalsent") {
            Some(v) => {
                queue.totalsent = Some(v.parse::<i32>().unwrap());
            },
            _ => queue.totalsent = None
        }

        queue
    }

    pub fn list_queues(con: &Connection) -> Vec<QueueLite> {
        let r: Vec<String> = cmd("SMEMBERS").arg("queues".to_string()).query(con).unwrap();
        let mut res: Vec<QueueLite> = Vec::new();
        for queue_name in r {
            res.push(QueueLite {
                name: queue_name
            })
        }

        res
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

    pub fn create_queue2(queue_info: QueueInfo, con: &Connection) -> QueueInfo {
        let mut queue_key = String::new();
        queue_key.push_str("queue:");
        queue_key.push_str(&queue_info.name);
        let now: DateTime<Utc> = Utc::now();
        let mut pipe = pipe();
        pipe.cmd("SADD").arg("queues".to_string()).arg(&queue_info.name).ignore();
        pipe.cmd("HMSET").arg(&queue_key)
            .arg("name".to_string())
            .arg(&queue_info.name)
            .arg("value".to_string())
            .arg(serde_json::to_string(&queue_info).unwrap())
            .arg("created_at".to_string())
            .arg(serde_json::to_string(&now).unwrap())
            .arg("totalrecv".to_string())
            .arg(0)
            .arg("totalsent".to_string())
            .arg(0).ignore();
        queue_key.push_str(":msg:counter");
        let _: Vec<String> = pipe.cmd("SET").arg(queue_key).arg(0).query(con).unwrap();

        queue_info
    }

    pub fn delete(queue_name: String, con: &Connection) -> bool {
        let mut match_queue_key = String::new();
        match_queue_key.push_str("queue:");
        match_queue_key.push_str(&queue_name);
        match_queue_key.push_str("*");

        let iter : Iter<String> = cmd("SCAN").cursor_arg(0).arg("MATCH").arg(match_queue_key).iter(con).unwrap();
        for key in iter {
            info!("DK: {:?}", key);
            let _: () = cmd("DEL").arg(key).query(con).unwrap();
        }

        true
    }
}
