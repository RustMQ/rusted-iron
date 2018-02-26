extern crate redis;

use redis::*;
use queue::Queue;

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<i32>,
    pub body: String
}

const MESSAGE_PART_KEY: &'static str = "msg";
const MESSAGE_GEN_PART_KEY: &'static str = "counter";

impl Message {
    pub fn push_message(queue: &Queue, message: &Message, con: &Connection) -> i32 {
        println!("Message: {:?}", message);
        let msg_counter_key = "queue:1:msg:counter";
        let queue_key = queue.get_queue_key();
        let mut msg_key = String::new();
        msg_key.push_str(&queue_key);
        msg_key.push_str(":msg:");

        let msg_id = redis::transaction(con, &[msg_counter_key], |pipe| {
            let msg_id: i32 = cmd("GET").arg(msg_counter_key).query(con).unwrap();
            msg_key.push_str(&msg_id.to_string());

            let response: Result<Vec<i32>, RedisError> = pipe
                    .cmd("HSET")
                        .arg(&msg_key)
                        .arg("body")
                        .arg(&message.body)
                    .cmd("INCR")
                        .arg("queue:1:msg:counter")
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
}