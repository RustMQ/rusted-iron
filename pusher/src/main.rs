extern crate redis;
extern crate reqwest;
extern crate serde_json;
extern crate queue;

use redis::Client;
use std::{env, thread};
use std::collections::HashMap;
use reqwest::header::{Headers, UserAgent};

use queue::{
    queue_info::{PushInfo, QueueSubscriber},
    message::PushMessage
};

fn construct_headers(headers: HashMap<String, String>) -> Headers {
    let mut result = Headers::new();
    result.set(UserAgent::new("pusher/0.1.0"));

    for (key, value) in &headers {
       result.set_raw(key.to_owned(), value.to_owned());
    }

    result
}

fn main() {
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
                println!("channel '{}': {}", msg.get_channel_name(), payload);
                let pm: PushMessage = serde_json::from_str(&payload).unwrap();
                let (msg_body, subscribers) =  {
                    let push_info: Option<PushInfo> = pm.queue_info.push.clone();
                    let subscribers: Vec<QueueSubscriber> = match push_info {
                        Some(pi) => {
                            pi.subscribers
                        },
                        None => Vec::new(),
                    };


                    (pm.msg_body, subscribers)
                };

                for subscriber in subscribers {
                    println!("Subscriber: {:?}", subscriber.url);
                    println!("MSG: {:?}", msg_body);
                    let reqwest_client = reqwest::Client::new();
                    let content = msg_body.clone();
                    let headers = subscriber.headers.unwrap();
                    let res = reqwest_client.post(subscriber.url.as_str())
                        .headers(construct_headers(headers))
                        .body(content)
                        .send().unwrap();
                    println!("Posted: {:?}", res);
                }
            }).unwrap();
        }
    }
}
