extern crate redis;
extern crate serde_json;

use objectid::{ObjectId};
use redis::*;
use serde_redis::RedisDeserialize;
use api::message::MessageDeleteBodyRequest;
use mq::queue::*;
use queue::{
    message::{Message, MessageState, PushMessage},
    queue::Queue,
    queue_info::{QueueInfo, QueueType, PushStatus}
};
use failure::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReserveMessageParams {
    pub n: i32,
    pub delete: Option<bool>
}

pub const MAXIMUM_NUMBER_TO_PEEK: i32 = 1;

pub fn push_message(queue_name: String, message: Message, con: &Connection) -> Result<String, Error> {
    let queue_key: String = Queue::get_queue_key(&queue_name);

    let mut queue_unreserved_key = String::new();
    queue_unreserved_key.push_str(&queue_key.clone());
    queue_unreserved_key.push_str(":unreserved:msg");

    let msg_counter_key = get_message_counter_key(&queue_name);
    let mut msg_key = String::new();
    msg_key.push_str(&queue_key);
    msg_key.push_str(":msg:");

    let mut msg = Message::with_body(&message.body);
    let msg_id = redis::transaction(con, &[&msg_counter_key], |pipe| {
        let msg_id: i32 = con.get(&msg_counter_key)?;
        let id = ObjectId::new().unwrap();
        msg_key.push_str(&id.to_string());
        msg.source_msg_id = Some(id.clone().to_string());
        let _response = pipe
                .atomic()
                .cmd("HMSET")
                    .arg(&msg_key)
                    .arg("body")
                    .arg(&message.body)
                    .arg("id")
                    .arg(&id.to_string())
                    .arg("source_msg_id")
                    .arg(&id.to_string())
                    .arg("state")
                    .arg(&msg.state.clone().unwrap().to_string())
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
                    .arg("size")
                    .arg(1)
                    .ignore()
                .cmd("HINCRBY")
                    .arg(&queue_key)
                    .arg("total_messages")
                    .arg(1)
                    .ignore()
                .query(con)?;

        Ok(Some(id.to_string()))
    }).unwrap();

    msg.id = Some(msg_id.clone());
    let queue = get_queue(&queue_name, &con)?;
    let qi_as_string = queue.value.expect("Queue Info should be present");

    let qi: QueueInfo = serde_json::from_str(qi_as_string.as_str())?;
    let queue_type = qi.queue_type.clone().unwrap();
    if queue_type == QueueType::Unicast || queue_type == QueueType::Multicast {
        let pm: PushMessage = PushMessage {
            queue_info: qi,
            msg: msg
        };
        let mut msg_channel_key = String::new();
        msg_channel_key.push_str(&queue_key.clone());
        msg_channel_key.push_str(":msg:channel");

        cmd("PUBLISH")
            .arg(&msg_channel_key)
            .arg(serde_json::to_string(&pm).unwrap())
            .execute(con);
    }

    Ok(msg_id)
}

pub fn get_message(queue_id: &String, message_id: &String, con: &Connection) -> Result<Message, Error> {
    let queue_key = Queue::get_queue_key(queue_id);
    let mut msg_key = String::new();
    msg_key.push_str(&queue_key);
    msg_key.push_str(":msg:");
    msg_key.push_str(&message_id.to_string());
    let v: Value = con.hgetall(msg_key)?;

    Ok(v.deserialize()?)
}

pub fn delete_message(queue_name: &String, message: &Message, con: &Connection) -> Result<bool, Error> {
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
        .hincr(&queue_key, "size", -1).ignore()
        .query(con)?;

    Ok(true)
}

pub fn reserve_messages(queue_name: &String, reserve_params: &ReserveMessageParams, con: &Connection) -> Result<Vec<Message>, Error> {
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
    let mut reserved_msg_list = Vec::new();
    let mut unreserved_msg_list = Vec::new();
    match unreserved_msg_key_list {
        Ok(unreserved_msg_key_list) => {
            for msg_key in unreserved_msg_key_list {
                reserved_msg_list.push((msg_key.1.clone(), msg_key.0.clone()));
                unreserved_msg_list.push(msg_key.0.clone());
            };
        },
        Err(err) => println!("Error: {:?}", err),
    }

    if reserved_msg_list.is_empty() {
        return Ok(result)
    }

    // 1. TX unreserved -> reserved
    let _r: Vec<isize> = redis::transaction(con, &[&queue_unreserved_key, &queue_reserved_key], |pipe| {
        pipe
            .atomic()
            .zrem(&queue_unreserved_key.clone(), unreserved_msg_list.clone()).ignore()
            .zadd_multiple(&queue_reserved_key.clone(), &reserved_msg_list)
            .query(con)
    }).unwrap();

    // 2. Loop over reserved (per n in request)
    for unreserved_msg_key in unreserved_msg_list.clone() {
        let id: ObjectId = ObjectId::new().unwrap();
        let _r: Vec<isize> = redis::transaction(con, &[unreserved_msg_key.clone()], |pipe| {
    // 2.2. update msg --> TX (?)
            pipe
                .atomic()
                .hset_nx(unreserved_msg_key.clone(), "reservation_id", id.to_string())
                .hset(unreserved_msg_key.clone(), "state", MessageState::Reserved.to_string()).ignore()
                .hincr(unreserved_msg_key.clone(), "reserved_count", 1).ignore()
                .query(con)
        }).unwrap();
    };
    // 3. collect updated msgs
    for updated_msg_key in unreserved_msg_list.clone() {
        let v: Value = con.hgetall(&updated_msg_key)?;
        result.push(v.deserialize()?);
    };

    if reserve_params.delete == Some(true) {
        for message in result.clone() {
            let _ = delete_message(&queue_name, &message, con)?;
        }
    }

    Ok(result)
}

pub fn delete(queue_name: String, message_id: String, con: &Connection) -> Result<bool, Error> {
    let m = Message {
        id: Some(message_id),
        body: String::new(),
        delay: None,
        reservation_id: None,
        reserved_count: None,
        source_msg_id: None,
        state: None
    };

    delete_message(&queue_name, &m, con)
}

pub fn delete_messages(queue_name: String, messages: &Vec<MessageDeleteBodyRequest>, con: &Connection) -> Result<Vec<bool>, Error> {
    Ok(messages
        .into_iter()
        .map(|message| {
            let deleted = delete(queue_name.to_string(), message.id.to_owned(), con).unwrap();
            deleted
        })
        .collect())
}

pub fn touch_message(queue_id: &String, message_id: &String, reservation_id: &String, con: &Connection) -> Result<String, Error> {
    let queue_key = Queue::get_queue_key(queue_id);
    let mut msg_key = String::new();
    msg_key.push_str(&queue_key);
    msg_key.push_str(":msg:");
    msg_key.push_str(&message_id.to_string());

    let msg = get_message(&queue_id, &message_id, con)?;

    let current_reservation_id = match msg.reservation_id {
        Some(reservation_id) => reservation_id,
        None => String::new(),
    };

    if current_reservation_id != reservation_id.to_string() {
        bail!("Touch message was failed");
    }

    let id: ObjectId = ObjectId::new().unwrap();
    let _: isize = con.hset(msg_key, "reservation_id", id.to_string())?;

    Ok(id.to_string())
}

pub fn peek_messages(queue_name: &String, number_to_peek: &i32, con: &Connection) -> Result<Vec<Message>, Error> {
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
        let v: Value = con.hgetall(&msg_key)?;
        result.push(v.deserialize()?);
    };

    Ok(result)
}

pub fn release_message(queue_name: &String, message_id: &String, reservation_id: &String, con: &Connection) -> Result<bool, Error> {
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

    let msg = get_message(&queue_name, &message_id, con)?;

    let current_reservation_id = match msg.reservation_id {
        Some(reservation_id) => reservation_id,
        None => String::new(),
    };
    if current_reservation_id != reservation_id.to_string() {
        return Ok(false)
    }

    let msg_score: i32 = con.zscore(&queue_reserved_key, &msg_key)?;

    let mut pipe = pipe();

    let res: i32 = pipe
        .zrem(&queue_reserved_key, &msg_key).ignore()
        .zadd(&queue_unreserved_key, &msg_key, msg_score).ignore()
        .hset(&msg_key, "state", MessageState::Unreserved.to_string()).ignore()
        .hdel(&msg_key, "reservation_id")
        .query(con)?;

    if res == 1 {
        return Ok(true)
    } else {
        return Ok(false)
    }
}

pub fn clear_messages(queue_name: &String, con: &Connection) -> Result<bool, Error> {
    let mut queue_key = String::new();
    queue_key.push_str("queue:");
    queue_key.push_str(&queue_name);

    let mut queue_msg_counter_key = String::new();
    queue_msg_counter_key.push_str(&queue_key.clone());
    queue_msg_counter_key.push_str(":msg:counter");

    let mut queue_unreserved_key = String::new();
    queue_unreserved_key.push_str(&queue_key.clone());
    queue_unreserved_key.push_str(":unreserved:msg");

    let mut queue_reserved_key = String::new();
    queue_reserved_key.push_str(&queue_key.clone());
    queue_reserved_key.push_str(":reserved:msg");

    let mut match_queue_key = String::new();
    match_queue_key.push_str(&queue_key.clone());
    match_queue_key.push_str(":msg*");

    let iter : Iter<String> = cmd("SCAN").cursor_arg(0).arg("MATCH").arg(match_queue_key).iter(con).unwrap();
    for key in iter {
        info!("DK: {:?}", key);
        if queue_msg_counter_key == key {
            continue;
        }
        let _: () = cmd("DEL").arg(key).query(con).unwrap();
    }

    let _ : () = con.del(queue_unreserved_key)?;
    let _ : () = con.del(queue_reserved_key)?;

    Ok(true)
}

pub fn get_push_statuses(queue_name: &String, message_id: &String, con: &Connection) -> Result<Vec<PushStatus>, Error> {
    let mut scan_key = String::new();
    scan_key.push_str("queue:");
    scan_key.push_str(queue_name);
    scan_key.push_str(":msg:");
    scan_key.push_str(message_id);
    scan_key.push_str(":delivery:*");

    let mut result = Vec::new();

    let iter : Iter<String> = cmd("SCAN").cursor_arg(0).arg("MATCH").arg(scan_key).iter(con)?;
    for key in iter {
        let push_status: String = con.hget(&key, "push_status")?;
        let v: PushStatus = serde_json::from_str(push_status.as_str())?;
        result.push(v);
    }

    Ok(result)
}
