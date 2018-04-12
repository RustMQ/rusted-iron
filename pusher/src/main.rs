extern crate redis;

use redis::Client;
use std::{env, thread};

fn main() {
    let database_url = env::var("REDISCLOUD_URL").expect("$REDISCLOUD_URL is provided");
    let client = Client::open(database_url.as_str()).expect("Failed to obtain client");
    let mut pubsub = client.get_pubsub().expect("Failed to obtain pub-sub");
    pubsub.psubscribe("queue:*:msg:channel").expect("Failed to subscribe on channel_1");

    loop {
        let msg = pubsub.get_message().unwrap();
        let payload : String = msg.get_payload().unwrap();
        if !payload.is_empty() {
            let builder = thread::Builder::new();
            builder.spawn(move || {
                println!("channel '{}': {}", msg.get_channel_name(), payload);
            }).unwrap();
        }
    }
}
