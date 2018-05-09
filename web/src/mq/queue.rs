extern crate serde_json;

use chrono::prelude::*;
use redis::{Commands, Connection, Iter, Value, cmd, pipe};
use serde_redis::RedisDeserialize;
use mq::message::{
    push_message
};
use queue::{
    message::*,
    queue::{Queue, QueueLite},
    queue_info::{
        QueueInfo,
        QueueSubscriber,
        PushInfo,
        QueueType
    }
};
use std::collections::HashMap;
use failure::Error;

pub fn list_queues(con: &Connection) -> Result<Vec<QueueLite>, Error> {
    let r: Vec<String> = con.smembers("queues")?;

    let mut res: Vec<QueueLite> = Vec::new();
    for queue_name in r {
        res.push(QueueLite {
            name: queue_name
        })
    }

    Ok(res)
}

pub fn get_queue(queue_name: &String, con: &Connection) -> Result<Queue, Error> {
    ensure!(!queue_name.trim().is_empty(), "Queue not found");

    let queue_key = Queue::get_queue_key(queue_name);
    let v: Value = con.hgetall(queue_key)?;

    Ok(v.deserialize()?)
}

pub fn get_message_counter_key(queue_id: &String) -> String {
    let mut key = String::new();
    let queue_key = Queue::get_queue_key(queue_id);
    key.push_str(&queue_key);
    key.push_str(":msg:counter");

    key
}

pub fn post_message(queue: Queue, message: Message, con: &Connection) -> Result<i32, Error> {
    Ok(push_message(queue.clone(), message, con)?)
}

pub fn create_queue(queue_info: QueueInfo, con: &Connection) -> QueueInfo {
    let mut queue_key = String::new();
    queue_key.push_str("queue:");
    let queue_name = queue_info.name.clone().unwrap();
    queue_key.push_str(&queue_name);
    let now: DateTime<Utc> = Utc::now();
    let mut pipe = pipe();
    pipe.cmd("SADD").arg("queues".to_string()).arg(&queue_name).ignore();
    pipe.cmd("HMSET").arg(&queue_key)
        .arg("name".to_string())
        .arg(&queue_name)
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
            let _ = create_queue(qi, con);
        }
    }

    queue_info
}

pub fn delete(queue_name: String, con: &Connection) -> Result<bool, Error> {
    let mut match_queue_key = String::new();
    match_queue_key.push_str("queue:");
    match_queue_key.push_str(&queue_name);
    match_queue_key.push_str("*");

    let iter : Iter<String> = cmd("SCAN").cursor_arg(0).arg("MATCH").arg(match_queue_key).iter(con)?;
    for key in iter {
        info!("DK: {:?}", key);
        let _: () = con.del(key)?;
    }

    Ok(true)
}

pub fn get_queue_info(queue_name: String, con: &Connection) -> Result<QueueInfo, Error> {
    let queue = get_queue(&queue_name, con)?;
    let queue_info_as_str = match queue.value {
        Some(v) => v,
        None => String::new()
    };

    let queue_info: QueueInfo = serde_json::from_str(&queue_info_as_str).unwrap();

    return Ok(queue_info);
}

pub fn update_subscribers(queue_name: String, mut new_subscribers: Vec<QueueSubscriber>, con: &Connection) -> bool {
    let queue_info_res = get_queue_info(queue_name, con);
    let mut current_subscribers;
    let mut queue_info = queue_info_res.unwrap();
    match queue_info.clone().push {
        Some(push) => {
            current_subscribers = push.subscribers.unwrap();
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
                subscribers: Some(subscribers),
                error_queue: push.error_queue
            };

        queue_info.push = Some(new_push);

        return update_queue_info(queue_info, con)
    }

    false
}

pub fn update_queue_info(queue_info: QueueInfo, con: &Connection) -> bool {
    let mut queue_key = String::new();
    queue_key.push_str("queue:");
    queue_key.push_str(queue_info.name.clone().unwrap().as_str());

    let _: () = cmd("HSET")
        .arg(queue_key)
        .arg("value")
        .arg(serde_json::to_string(&queue_info).unwrap())
        .query(con).unwrap();

    true
}

pub fn replace_subscribers(queue_name: String, new_subscribers: Vec<QueueSubscriber>, con: &Connection) -> bool {
    let queue_info_res = get_queue_info(queue_name, con);
    let mut queue_info = queue_info_res.unwrap();
    if queue_info.push.is_some() {
        let push = queue_info.push.unwrap();

        let new_push = PushInfo {
                retries_delay: push.retries_delay,
                retries: push.retries,
                subscribers: Some(new_subscribers),
                error_queue: push.error_queue
        };

        queue_info.push = Some(new_push);

        return update_queue_info(queue_info, con)
    }

    false
}

pub fn delete_subscribers(queue_name: String, subscribers_for_delete: Vec<QueueSubscriber>, con: &Connection) -> bool {
    let queue_info_res = get_queue_info(queue_name.clone(), con);
    let current_subscribers;
    let queue_info = queue_info_res.unwrap();
    match queue_info.clone().push {
        Some(push) => {
            current_subscribers = push.subscribers.unwrap();
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

    replace_subscribers(queue_name, subscribers_for_update, con)
}

pub fn patch_queue_info(queue_name: String, queue_info_patch: QueueInfo, con: &Connection) -> Result<QueueInfo, Error> {
    let mut current_queue_info = get_queue_info(queue_name.clone(), &con)?;
    info!("PATCH QUEUE: {:#?}", queue_info_patch);

    if queue_info_patch.message_timeout.is_some() {
        current_queue_info.message_timeout = queue_info_patch.message_timeout;
    }
    if queue_info_patch.message_expiration.is_some() {
        current_queue_info.message_expiration = queue_info_patch.message_expiration;
    }

    if current_queue_info.queue_type == Some(QueueType::Pull) && queue_info_patch.push.is_some() {
        bail!("Queue type cannot be changed")
    }

    if current_queue_info.queue_type == Some(QueueType::Unicast) || current_queue_info.queue_type == Some(QueueType::Multicast) {
        let mut new_push = PushInfo {
            retries_delay: None,
            retries: None,
            subscribers: None,
            error_queue: None
        };
        if queue_info_patch.push.is_some() {
            let current_push = current_queue_info.push.unwrap();
            let push = queue_info_patch.push.unwrap();
            if push.retries.is_some() {
                new_push.retries = push.retries;
            } else {
                new_push.retries = current_push.retries;
            }
            if push.retries_delay.is_some() {
                new_push.retries_delay = push.retries_delay;
            } else {
                new_push.retries_delay = current_push.retries_delay;
            }
            if push.error_queue.is_some() {
                if current_push.error_queue != push.error_queue {
                    let qi = QueueInfo::new(push.error_queue.unwrap());
                    let _ = create_queue(qi, con);
                }
            } else {
                new_push.error_queue = current_push.error_queue;
            }
            if push.subscribers.is_some() {
                if !update_subscribers(queue_name, push.subscribers.clone().unwrap(), con) {
                    bail!("Bad request");
                }

                new_push.subscribers = push.subscribers;
            } else {
                new_push.subscribers = current_push.subscribers;
            }
            current_queue_info.push = Some(new_push);
        }
    }

    update_queue_info(current_queue_info.clone(), con);

    Ok(current_queue_info)
}
