use std::collections::HashMap;
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

    fn get_queue_key(queue_id: i32) -> String {
        let mut key = String::new();
        key.push_str(DEFAULT_QUEUE_KEY);
        key.push_str(&queue_id.to_string());

        key.to_string()
    }

    fn new_from_hash(queue_id: i32, hash_map: HashMap<String, String>) -> Queue {
        let mut queue = Queue::new();
        queue.id = Some(queue_id);

        match hash_map.get(&*"name") {
            Some(v) => {
                queue.name = Some(v.to_string());
            },
            _ => println!("Wrong key"),
        }
        match hash_map.get(&*"class") {
            Some(v) => {
                queue.class = Some(v.to_string());
            },
            _ => println!("Wrong key"),
        }
        match hash_map.get(&*"totalrecv") {
            Some(v) => {
                queue.totalrecv = Some(v.parse::<i32>().unwrap());
            },
            _ => println!("Wrong key"),
        }
        match hash_map.get(&*"totalsent") {
            Some(v) => {
                queue.totalsent = Some(v.parse::<i32>().unwrap());
            },
            _ => println!("Wrong key"),
        }

        queue
    }

    pub fn get_counter_key() -> Result<String, QueueError> {
        let mut key = String::new();
        key.push_str(DEFAULT_QUEUE_KEY);
        key.push_str("1");

        Ok(key.to_string())
    }

    pub fn get_queue(queue_id: i32, con: &Connection) -> Queue {
        let key = Queue::get_queue_key(queue_id);
        let result: HashMap<String, String> = cmd("HGETALL").arg(key).query(con).unwrap();

        Queue::new_from_hash(queue_id, result)
    }
}