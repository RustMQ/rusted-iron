use redis::*;
use queue::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<i32>,
    pub body: String
}

const MESSAGE_PART_KEY: &'static str = "msg";
const MESSAGE_GEN_PART_KEY: &'static str = "counter";

impl Message {
    pub fn push_message(message: Message, _con: &Connection) -> Result<Message, RedisError> {
        // Step 1. INCR msg_counter queue:<queue_id>:msg:counter
        // Step 2. ADD message to set of reserved messages
        // Step 3. Update queue information
        //
        println!("Message: {:?}", message);
        let _ : Result<i32, RedisError> = cmd("SET").arg("my_super2").arg(42).query(_con);
        let m = Message {
            id: Some(1),
            body: message.body
        };
        println!("New Message: {:?}", m);
        Ok(m)
    }
}