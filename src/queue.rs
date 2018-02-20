#[derive(PartialEq, Eq, Clone, Debug, Copy)]
pub enum QueueError {
    CounterKeyMissing
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Queue {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub totalrecv: Option<i32>,
    pub totalsent: Option<i32>
}

pub const DEFAULT_QUEUE_KEY: &'static str = "queue";

impl Queue {
    pub fn get_counter_key() -> Result<String, QueueError> {
        let mut key = String::new();
        key.push_str(DEFAULT_QUEUE_KEY);
        key.push_str(":1");

        Ok(key.to_string())
    }
}