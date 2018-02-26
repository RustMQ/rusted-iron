use std::collections::HashMap;
use message::Message;
use redis::*;

#[derive(PartialEq, Eq, Clone, Debug, Copy)]
pub enum QueueError {
    CounterKeyMissing
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Queue {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub class: Option<String>,
    pub totalrecv: Option<i32>,
    pub totalsent: Option<i32>
}

pub const DEFAULT_QUEUE_KEY: &'static str = "queue:";

impl Queue {
    fn new() -> Queue {
        Queue {
            id: None,
            class: None,
            name: None,
            totalrecv: None,
            totalsent: None
        }
    }

    pub fn get_queue_key(&self) -> String {
        let queue_id = &self.id.expect("id value");
        let mut key = String::new();
        key.push_str(DEFAULT_QUEUE_KEY);
        key.push_str(&queue_id.to_string());

        key
    }

    fn new_from_hash(queue_id: i32, hash_map: HashMap<String, String>) -> Queue {
        let mut queue = Queue::new();
        queue.id = Some(queue_id);

        match hash_map.get(&*"name") {
            Some(v) => {
                queue.name = Some(v.to_string());
            },
            _ => println!("Wrong key name"),
        }
        match hash_map.get(&*"class") {
            Some(v) => {
                queue.class = Some(v.to_string());
            },
            _ => println!("Wrong key class"),
        }
        match hash_map.get(&*"totalrecv") {
            Some(v) => {
                queue.totalrecv = Some(v.parse::<i32>().unwrap());
            },
            _ => println!("Wrong key totalrecv"),
        }
        match hash_map.get(&*"totalsent") {
            Some(v) => {
                queue.totalsent = Some(v.parse::<i32>().unwrap());
            },
            _ => println!("Wrong key totalsent"),
        }

        queue
    }

    pub fn get_queue(queue_id: i32, con: &Connection) -> Queue {
        let mut q = Queue::new();
        q.id = Some(queue_id);
        let queue_key = q.get_queue_key();
        let result: HashMap<String, String> = cmd("HGETALL").arg(queue_key).query(con).unwrap();

        Queue::new_from_hash(queue_id, result)
    }

    pub fn get_message_counter_key(&self) -> String {
        let mut key = String::new();
        let queue_key = &self.get_queue_key();
        key.push_str(queue_key);
        key.push_str(":msg:counter");

        key
    }

    pub fn post_message(&self, message: &Message, con: &Connection) -> Result<i32, QueueError> {
        println!("Q: {:?}", &self);

        Ok(Message::push_message(&self, message, con))
    }
}