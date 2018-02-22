extern crate redis;

use redis::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<i32>,
    pub body: String
}

const MESSAGE_PART_KEY: &'static str = "msg";
const MESSAGE_GEN_PART_KEY: &'static str = "counter";

impl Message {
    pub fn push_message(message: &Message, _con: &Connection) -> i32 {
        // Step 1. INCR msg_counter queue:<queue_id>:msg:counter
        // Step 2. ADD message to set of reserved messages
        // Step 3. Update queue information
        //
        println!("Message: {:?}", message);
        let mid: i32 = cmd("GET").arg("queue:1:msg:counter").query(_con).unwrap();
        let mut key = String::new();
        key.push_str("queue:1:msg:");
        key.push_str(&mid.to_string());
        let response: Result<Vec<i32>, RedisError> = redis::pipe()
            .atomic()
            .cmd("HSET")
                .arg(key)
                .arg("body")
                .arg(&message.body)
            .cmd("INCR")
                .arg("queue:1:msg:counter")
            .cmd("HINCRBY")
                .arg("queue:1")
                .arg("totalrecv")
                .arg("1")
                .ignore()
            .query(_con);
        match response {
            Err(e) => print!("{:?}", e),
            Ok(v) => {
                println!("V: {:?}", v);
            }
        }

        1
    }
}