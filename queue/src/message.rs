use queue_info::QueueInfo;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub delay: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub reserved_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub reservation_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PushMessage {
    pub queue_info: QueueInfo,
    pub msg_body: String
}

impl Message {
    pub fn new(body: &str, delay: u32) -> Message {
        Message {
            body: String::from(body),
            delay: Some(delay),
            id: None,
            reserved_count: None,
            reservation_id: None,
        }
    }

    pub fn with_body(body: &str) -> Message {
        Message {
            body: String::from(body),
            delay: None,
            id: None,
            reserved_count: None,
            reservation_id: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReservationConfig {
    n: u8,
    timeout: u32,
    wait: u32,
    delete: bool,
}

impl ReservationConfig {
    pub fn new(n: u8, timeout: u32, wait: u32, delete: bool) -> ReservationConfig {
        ReservationConfig {
            n,
            timeout,
            wait,
            delete,
        }
    }
}
