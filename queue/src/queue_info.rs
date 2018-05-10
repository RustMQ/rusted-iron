use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum QueueType {
    Pull,
    Unicast,
    Multicast
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueueInfo {
    #[serde(skip_serializing_if = "Option::is_none")] pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub message_timeout: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub message_expiration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")] pub total_messages: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")] pub push: Option<PushInfo>,
    #[serde(skip_serializing_if = "Option::is_none")] pub alerts: Option<Vec<Alert>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub queue_type: Option<QueueType>,
}

impl QueueInfo {
    pub fn new(name: String) -> QueueInfo {
        QueueInfo {
            name: Some(name),
            project_id: None,
            message_timeout: None,
            message_expiration: None,
            queue_type: Some(QueueType::Pull),
            size: None,
            total_messages: None,
            push: None,
            alerts: None,
        }
    }

    pub fn default(name: String) -> QueueInfo {
        QueueInfo {
            name: Some(name),
            project_id: None,
            message_timeout: Some(60),
            message_expiration: Some(604800),
            queue_type: Some(QueueType::Pull),
            size: None,
            total_messages: None,
            push: None,
            alerts: None,
        }
    }

    pub fn name(&mut self, name: String) -> &mut QueueInfo {
        self.name = Some(name);

        self
    }

    pub fn message_timeout(&mut self, message_timeout: u32) -> &mut QueueInfo {
        self.message_timeout = Some(message_timeout);

        self
    }

    pub fn message_expiration(&mut self, message_expiration: u32) -> &mut QueueInfo {
        self.message_expiration = Some(message_expiration);

        self
    }

    pub fn queue_type(&mut self, queue_type: QueueType) -> &mut QueueInfo {
        self.queue_type = Some(queue_type);

        self
    }

    pub fn push(&mut self, push: PushInfo) -> &mut QueueInfo {
        self.push = Some(push);

        self
    }

    pub fn alerts(&mut self, alerts: Vec<Alert>) -> &mut QueueInfo {
        self.alerts = Some(alerts);

        self
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PushInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retries_delay: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retries: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribers: Option<Vec<QueueSubscriber>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_queue: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueueSubscriber {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

impl QueueSubscriber {
    pub fn new(name: &str, url: &str) -> QueueSubscriber {
        QueueSubscriber {
            name: String::from(name),
            url: Some(String::from(url)),
            headers: None
        }
    }

    pub fn headers(&mut self, headers: HashMap<String, String>) {
        self.headers = Some(headers);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Asc,
    Desc,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AlertType {
    Fixed,
    Progressive,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Alert {
    #[serde(rename = "type")]
    pub alert_type: AlertType,
    pub trigger: u32,
    pub queue: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub snooze: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub direction: Option<Direction>
}

impl Alert {
    pub fn new(alert_type: AlertType, trigger: u32, queue: &str) -> Alert {
        Alert {
            alert_type: alert_type,
            trigger: trigger,
            queue: String::from(queue),
            snooze: None,
            direction: None
        }
    }

    pub fn snooze(&mut self, snooze: u32) -> &mut Alert {
        self.snooze = Some(snooze);

        self
    }

    pub fn direction(&mut self, direction: Direction) -> &mut Alert {
        self.direction = Some(direction);

        self
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PushStatus {
    pub subscriber_name: String,
    pub retries_remaining: u32,
    pub tries: u32,
	pub status_code: Option<u16>,
	pub url: String,
    pub msg: Option<String>
}