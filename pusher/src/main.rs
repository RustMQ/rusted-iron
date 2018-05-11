extern crate redis;
extern crate reqwest;
#[macro_use]
extern crate serde_json;
extern crate serde;
extern crate queue;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate base64;
extern crate failure;

use redis::{Client, Commands};
use std::{
    collections::HashMap,
    env,
    thread,
    time::Duration
};
use reqwest::{
    header::{Headers, UserAgent, ContentType},
    Response,
    StatusCode
};
use queue::{
    queue_info::{PushStatus, PushInfo, QueueSubscriber, QueueType},
    message::{PushMessage, Message},
    queue::Queue
};
use base64::encode;
use failure::Error;

#[derive(Debug)]
struct Retry {
    retry_count: u32,
    retry_delay: u32
}

fn construct_headers(headers: HashMap<String, String>, skip_content_type: bool) -> Headers {
    let mut result = Headers::new();
    result.set(UserAgent::new("pusher/0.1.0"));
    if !skip_content_type {
        result.set(ContentType::json());
    }

    for (key, value) in &headers {
       result.set_raw(key.to_owned(), value.to_owned());
    }

    result
}

fn send_message_to_error_queue(message: Message, error_queue_name: String) {
    let web_api_url = env::var("WEB_API_URL").expect("$WEB_API_URL is provided");
    let path = format!("{}/queues/{}/messages", web_api_url, error_queue_name);
    info!("PATH: {:?}", path);

    let mut messages: Vec<Message> = Vec::new();
    messages.push(message);

    let reqwest_client = reqwest::Client::new();
    let body = json!(messages);
    let res = reqwest_client
        .post(path.as_str())
        .headers(construct_headers(HashMap::new(), false))
        .body(body.to_string().into_bytes())
        .send()
        .unwrap();

    debug!("Pushed to error_queue: {:#?}", res);
}

fn post_message_to_subscriber(message: Message, subscriber: QueueSubscriber) -> Response {
    info!("Subscriber: {:#?}", subscriber.url);
    info!("MSG: {:#?}", message.body);
    let reqwest_client = reqwest::Client::new();
    let content = message.body.clone();
    let headers = subscriber.headers.unwrap();
    let url = subscriber.url.unwrap();
    reqwest_client.post(url.as_str())
        .headers(construct_headers(headers, false))
        .body(content)
        .send().unwrap()
}

fn prepare_client() -> Client {
    let database_url = env::var("REDISCLOUD_URL").expect("$REDISCLOUD_URL is provided");
    Client::open(database_url.clone().as_str()).expect("Failed to obtain client")
}

fn update_push_status(push_message: PushMessage, subscriber: QueueSubscriber, status: StatusCode) -> Result<bool, Error> {
    let client = prepare_client();
    let connection = client.get_connection().unwrap();
    let msg_key = {
        let queue_name = {
            let queue_info = push_message.queue_info.clone();
            queue_info.name.unwrap()
        };
        let msg = push_message.msg.clone();
        let queue_key = Queue::get_queue_key(&queue_name);
        let mut msg_key = String::new();
        msg_key.push_str(&queue_key);
        msg_key.push_str(":msg:");
        msg_key.push_str(&msg.source_msg_id.unwrap());
        msg_key.push_str(":delivery:");
        let delivery_id = encode(&subscriber.url.clone().unwrap());
        msg_key.push_str(delivery_id.as_str());

        msg_key
    };
    let push_status = PushStatus {
        subscriber_name: subscriber.name,
        retries_remaining: 0,
        tries: 0,
	    status_code: Some(status.as_u16()),
	    url: subscriber.url.unwrap(),
        msg: Some(push_message.msg.body.clone())
    };

    let () = connection.hset(msg_key, "push_status", serde_json::to_string(&push_status).unwrap())?;

    Ok(true)
}

fn main() {
    env_logger::init();

    info!("pusher starting up");
    let client = prepare_client();
    let mut pubsub = client.get_pubsub().expect("Failed to obtain pub-sub");
    pubsub.psubscribe("queue:*:msg:channel").expect("Failed to subscribe on queue:*:msg:channel");

    loop {
        let msg = pubsub.get_message().unwrap();
        let payload : String = msg.get_payload().unwrap();
        if !payload.is_empty() {
            let builder = thread::Builder::new();
            builder.spawn(move || {
                info!("channel '{}': {}", msg.get_channel_name(), payload);
                let pm: PushMessage = serde_json::from_str(&payload).unwrap();
                let retry: Retry = {
                    let (retry_count, retry_delay) = match pm.queue_info.push.clone() {
                        Some(pi) => (pi.retries.unwrap(), pi.retries_delay.unwrap()),
                        None => (0, 0)
                    };
                    Retry {
                        retry_count,
                        retry_delay
                    }
                };

                let (msg, subscribers, queue_type, error_queue_name) =  {
                    let push_info: Option<PushInfo> = pm.queue_info.push.clone();
                    let (subscribers, error_queue_name): (Vec<QueueSubscriber>, String) = match push_info {
                        Some(pi) => {
                            (pi.subscribers.unwrap(), pi.error_queue.unwrap())
                        },
                        None => (Vec::new(), String::new()),
                    };


                    (pm.msg.clone(), subscribers, pm.queue_info.queue_type.clone(), error_queue_name)
                };

                let is_unicast_mode = |queue_type: Option<QueueType>| queue_type.unwrap() == QueueType::Unicast;
                let delay = Duration::from_secs(retry.retry_delay.into());
                for i in 1..=retry.retry_count {
                    info!("Retry num: {:#?}", i);
                    let mut break_retry = false;
                    for subscriber in subscribers.clone() {
                        let res = post_message_to_subscriber(msg.clone(), subscriber.clone());
                        match update_push_status(pm.clone(), subscriber.clone(), res.status()) {
                            Ok(_updated) => info!("Push status updated: {:?}", subscriber.url),
                            Err(e) => info!("Push status not updated: {:?}", e.to_string())
                        };
                        if res.status().is_success() {
                            info!("Message successfully sent.");
                            break_retry = true;
                            if is_unicast_mode(queue_type.clone()) {
                                break;
                            }
                        } else {
                            info!("Something else happened. Status: {:?}", res.status());
                            break_retry = false;
                        }
                    }

                    if break_retry == true {
                        info!("No retry is required");
                        break;
                    }
                    if retry.retry_count == i {
                        info!("No delivery. Moved to error_queue.");
                        if !error_queue_name.is_empty() {
                            let message = Message::with_body(payload.as_str());
                            send_message_to_error_queue(message, error_queue_name);
                        }

                        break;
                    }

                    info!("New try will be triggered soon");
                    thread::sleep(delay);
                }

            }).unwrap();
        }
    }
}
