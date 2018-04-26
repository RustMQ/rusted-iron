extern crate redis;
extern crate reqwest;
#[macro_use]
extern crate serde_json;
extern crate queue;
#[macro_use]
extern crate log;
extern crate env_logger;

use redis::Client;
use std::{
    collections::HashMap,
    env,
    thread,
    time::Duration
};
use reqwest::header::{Headers, UserAgent, ContentType};

use queue::{
    queue_info::{PushInfo, QueueSubscriber, QueueType},
    message::{PushMessage, Message}
};

#[derive(Debug)]
struct Retry {
    retry_count: u32,
    retry_delay: u32
}

fn construct_headers(headers: HashMap<String, String>) -> Headers {
    let mut result = Headers::new();
    result.set(UserAgent::new("pusher/0.1.0"));

    for (key, value) in &headers {
       result.set_raw(key.to_owned(), value.to_owned());
    }

    result
}

fn main() {
    env_logger::init();

    info!("pusher starting up");
    let database_url = env::var("REDISCLOUD_URL").expect("$REDISCLOUD_URL is provided");
    let client = Client::open(database_url.as_str()).expect("Failed to obtain client");
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
                        Some(pi) => (pi.retries, pi.retries_delay),
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
                            (pi.subscribers, pi.error_queue.unwrap())
                        },
                        None => (Vec::new(), String::new()),
                    };


                    (pm.msg, subscribers, pm.queue_info.queue_type, error_queue_name)
                };

                let is_unicast_mode = |queue_type: Option<QueueType>| queue_type.unwrap() == QueueType::Unicast;
                let delay = Duration::from_secs(retry.retry_delay.into());
                for i in 1..retry.retry_count + 1 {
                    info!("Retry num: {:#?}", i);
                    let mut break_retry = false;
                    for subscriber in subscribers.clone() {
                        info!("Subscriber: {:#?}", subscriber.url);
                        info!("MSG: {:#?}", msg.body);
                        let reqwest_client = reqwest::Client::new();
                        let content = msg.body.clone();
                        let headers = subscriber.headers.unwrap();
                        let url = subscriber.url.unwrap();
                        let res = reqwest_client.post(url.as_str())
                            .headers(construct_headers(headers))
                            .body(content)
                            .send().unwrap();

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
                        if error_queue_name.is_empty() {
                            break;
                        }

                        let reqwest_client = reqwest::Client::new();
                        let mut headers = Headers::new();
                        headers.set(ContentType::json());
                        headers.set(UserAgent::new("pusher/0.1.0"));
                        let web_api_url = env::var("WEB_API_URL").expect("$WEB_API_URL is provided");
                        let path = format!("{}/queues/{}/messages", web_api_url, error_queue_name);
                        info!("PATH: {:?}", path);
                        let mut messages: Vec<Message> = Vec::new();
                        let mut m = Message::new(payload.as_str(), 0);
                        messages.push(m);
                        let body = json!(messages);
                        let res = reqwest_client
                            .post(path.as_str())
                            .headers(headers)
                            .body(body.to_string().into_bytes())
                            .send()
                            .unwrap();

                        debug!("Pushed to error_queue: {:#?}", res);
                        break;
                    }

                    info!("New try will be triggered soon");
                    thread::sleep(delay);
                }

            }).unwrap();
        }
    }
}
