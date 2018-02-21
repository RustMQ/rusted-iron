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
    pub fn get_counter_key() -> Result<String, QueueError> {
        let mut key = String::new();
        key.push_str(DEFAULT_QUEUE_KEY);
        key.push_str("1");

        Ok(key.to_string())
    }

    pub fn get_queue(queue_id: i32, con: &Connection) -> Queue {
        let mut key = String::new();
        key.push_str(DEFAULT_QUEUE_KEY);
        key.push_str(&queue_id.to_string());
        let mut queue = Queue::new();

        let result: HashMap<String, String> = cmd("HGETALL").arg(key.to_string()).query(con).unwrap();
        println!("GET_G: {:?}", result);

        match result.get(&*"class") {
            Some(v) => {
                println!("v: {}", v);
                queue.class = Some(v.to_string());
            },
            _ => println!("Wrong key"),
        }
        match result.get(&*"totalrecv") {
            Some(v) => {
                println!("v: {}", v);
                queue.totalrecv = Some(v.parse::<i32>().unwrap());
            },
            _ => println!("Wrong key"),
        }
        match result.get(&*"totalsent") {
            Some(v) => {
                println!("v: {}", v);
                queue.totalsent = Some(v.parse::<i32>().unwrap());
            },
            _ => println!("Wrong key"),
        }

        println!("GET_Q: {:?}", queue);
        queue
    }
}