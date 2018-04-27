extern crate serde_json;

use chrono::prelude::*;
use redis::*;
use serde_redis::RedisDeserialize;
use mq::message::Message;
use queue::queue_info::{
    QueueInfo,
    QueueSubscriber,
    PushInfo,
};
use std::collections::HashMap;
use failure::Error;

#[derive(PartialEq, Eq, Clone, Debug, Copy)]
pub enum QueueError {
    CounterKeyMissing
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Queue {
    pub name: Option<String>,
    pub value: Option<String>,
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
            name: None,
            value: None,
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
        let v: Result<Value, RedisError> = con.hgetall(queue_key);

        let result: Queue = match v {
            Ok(v) => {
                v.deserialize().unwrap()
            },
            Err(e) => {
                debug!("qet_queue error: {:?}", e);
                Queue::new()
            },
        };

        result
    }

    pub fn get_message_counter_key(queue_id: &String) -> String {
        let mut key = String::new();
        let queue_key = Queue::get_queue_key(queue_id);
        key.push_str(&queue_key);
        key.push_str(":msg:counter");

        key
    }

    pub fn post_message(queue: Queue, message: Message, con: &Connection) -> Result<i32, QueueError> {
        Ok(Message::push_message(queue.clone(), message, con))
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
            value: Some("pull".to_string()),
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

        if queue_info.clone().push.is_some() {
            let push = queue_info.clone().push.unwrap();
            if !push.error_queue.is_none() {
                let qi = QueueInfo::new(push.error_queue.unwrap());
                let _ = Queue::create_queue2(qi, con);
            }
        }

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

    pub fn get_queue_info(queue_name: String, con: &Connection) -> Result<QueueInfo, Error> {
        let queue = Queue::get_queue(&queue_name, con);
        let queue_info_as_str = match queue.value {
            Some(v) => v,
            None => String::new()
        };

        let queue_info: QueueInfo = serde_json::from_str(&queue_info_as_str).unwrap();
        return Ok(queue_info);
    }

    pub fn update_subscribers(queue_name: String, mut new_subscribers: Vec<QueueSubscriber>, con: &Connection) -> bool {
        let queue_info_res = Queue::get_queue_info(queue_name, con);
        let mut current_subscribers;
        let mut queue_info = queue_info_res.unwrap();
        match queue_info.clone().push {
            Some(push) => {
                current_subscribers = push.subscribers;
            },
            None => {
                info!("Broken subscribers!");
                current_subscribers = Vec::new();
            },
        };
        current_subscribers.append(&mut new_subscribers);
        let unique_subscribers: HashMap<_, _> = current_subscribers.iter()
            .map(|subscriber| (subscriber.name.clone(), subscriber))
            .collect();
        if queue_info.push.is_some() {
            let push = queue_info.push.unwrap();
            let mut subscribers = Vec::new();
            for (_, val) in unique_subscribers {
                subscribers.push(val.clone());
            }

            let new_push = PushInfo {
                    retries_delay: push.retries_delay,
                    retries: push.retries,
                    subscribers: subscribers,
                    error_queue: push.error_queue
                };

            queue_info.push = Some(new_push);

            return Queue::update_queue_info(queue_info, con)
        }

        false
    }

    pub fn update_queue_info(queue_info: QueueInfo, con: &Connection) -> bool {
        let mut queue_key = String::new();
        queue_key.push_str("queue:");
        queue_key.push_str(&queue_info.name);
        let _: () = cmd("HSET")
            .arg(queue_key)
            .arg("value")
            .arg(serde_json::to_string(&queue_info).unwrap())
            .query(con).unwrap();


        true
    }

    pub fn replace_subscribers(queue_name: String, new_subscribers: Vec<QueueSubscriber>, con: &Connection) -> bool {
        let queue_info_res = Queue::get_queue_info(queue_name, con);
        let mut queue_info = queue_info_res.unwrap();
        if queue_info.push.is_some() {
            let push = queue_info.push.unwrap();

            let new_push = PushInfo {
                    retries_delay: push.retries_delay,
                    retries: push.retries,
                    subscribers: new_subscribers,
                    error_queue: push.error_queue
            };

            queue_info.push = Some(new_push);

            return Queue::update_queue_info(queue_info, con)
        }

        false
    }

    pub fn delete_subscribers(queue_name: String, subscribers_for_delete: Vec<QueueSubscriber>, con: &Connection) -> bool {
        let queue_info_res = Queue::get_queue_info(queue_name.clone(), con);
        let current_subscribers;
        let queue_info = queue_info_res.unwrap();
        match queue_info.clone().push {
            Some(push) => {
                current_subscribers = push.subscribers;
            },
            None => {
                info!("Broken subscribers!");
                current_subscribers = Vec::new();
            },
        };
        if current_subscribers.len() == 1 {
            return false;
        }
        let subscribers_for_delete_as_map: HashMap<_, _> = subscribers_for_delete.iter()
            .map(|s| (s.name.clone(), s))
            .collect();

        let subscribers_for_update: Vec<QueueSubscriber> = current_subscribers.into_iter()
            .filter(|subscriber| !subscribers_for_delete_as_map.contains_key(&subscriber.name))
            .collect();

        Queue::replace_subscribers(queue_name, subscribers_for_update, con)
    }
}
