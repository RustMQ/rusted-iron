#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Queue {
    pub name: Option<String>,
    pub value: Option<String>,
    pub size: Option<usize>,
    pub total_messages: Option<usize>
}

#[derive(Serialize, Deserialize)]
pub struct QueueLite {
    pub name: String
}

pub const DEFAULT_QUEUE_KEY: &'static str = "queue:";

impl Queue {
    pub fn new() -> Queue {
        Queue {
            name: None,
            value: None,
            size: None,
            total_messages: None
        }
    }

    pub fn get_queue_key(queue_id: &String) -> String {
        let mut key = String::new();
        key.push_str(DEFAULT_QUEUE_KEY);
        key.push_str(&queue_id);

        key
    }
}