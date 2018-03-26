extern crate futures;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate hyper;
extern crate mime;

extern crate redis;
extern crate r2d2;
extern crate r2d2_redis;
extern crate serde;
#[macro_use]
extern crate serde_json;
extern crate objectid;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rayon;

mod redis_api;
mod redis_middleware;
mod db;
mod queue;
mod message;
// #[cfg(test)] mod tests;

use std::env;

use hyper::{Response, StatusCode};

use gotham::router::Router;
use gotham::router::builder::*;
use gotham::http::response::create_response;
use gotham::state::State;
use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;

// use message::{Message, ReserveMessageParams};
use db::{pool, Pool};

use redis_middleware::RedisMiddleware;
use queue::QueuePathExtractor;


fn router(pool: Pool) -> Router {
    let redis_middleware = RedisMiddleware::with_pool(pool);

    let (chain, pipelines) = single_pipeline(
        new_pipeline().add(redis_middleware).build()
    );

    build_router(chain, pipelines, |route| {
        route.get("/").to(index);

        route.scope("/redis", |route| {
            route.get("/version").to(redis_api::version);
        });

        route.get("/queues").to(queue::list_queues);

        route.scope("/queues/:name", |route| {
            route.put("")
                .with_path_extractor::<QueuePathExtractor>()
                .to(queue::put_queue);
            route
                .post("/messages")
                .with_path_extractor::<QueuePathExtractor>()
                .to(queue::push_messages);
            route
                .post("/reservations")
                .with_path_extractor::<QueuePathExtractor>()
                .to(queue::reserve_messages);
            route.delete("")
                .with_path_extractor::<QueuePathExtractor>()
                .to(queue::delete_queue);
        });
    })
}


pub fn index(state: State) -> (State, Response) {
    let res = {
        let res_str = r#"{
            "goto": "http://www.iron.io"
        }"#;
        create_response(
            &state,
            StatusCode::Ok,
            Some((
                res_str.to_string().into_bytes(),
                mime::APPLICATION_JSON
            )),
        )
    };

    (state, res)
}
/*

#[get("/<queue_id>", format = "application/json")]
fn get_queue_info(queue_id: String, _conn: RedisConnection) -> Json<Value> {
    let q = Queue {
        id: Some(queue_id),
        class: Some(String::from("pull")),
        name: None,
        totalrecv: None,
        totalsent: None
    };

    return Json(json!(q))
}

#[post("/<queue_id>/messages", format = "application/json", data = "<messages>")]
fn post_message_to_queue(
    queue_id: String,
    messages: Json<Vec<Message>>,
    conn: RedisConnection
) -> Json<Value> {
    let q: Queue = Queue::get_queue(&queue_id, &*conn);
    println!("Q: {:?}", q);
    let mut result = Vec::new();
    for x in messages.0 {
        let mut m: Message = Message::new();
        m.body = x.body;
        let mid = Queue::post_message(q.clone(), m, &*conn).expect("Message put on queue.");
        result.push(mid.to_string());
    };

    return Json(json!({
        "ids": result,
        "msg": String::from("Messages put on queue.")
    }))
}

#[get("/<queue_id>/messages/<message_id>", format = "application/json")]
fn get_message_from_queue(
    queue_id: String,
    message_id: String,
    conn: RedisConnection
) -> Json<Value> {
    let m: Message = Queue::get_message(&queue_id, &message_id, &*conn).expect("Message return");

    return Json(json!({
        "message": m
    }))
}

#[delete("/<queue_id>/messages/<message_id>", format = "application/json")]
fn delete_message_from_queue(
    queue_id: String,
    message_id: String,
    conn: RedisConnection
) -> Option<Json<Value>> {
    if Queue::delete_message(&queue_id, &message_id, &*conn) {
        return Some(Json(json!({
            "msg": "Deleted"
        })))
    }
    else {
        return None
    }
}

#[post("/<queue_id>/reservations", format = "application/json", data="<reserve_params>")]
fn reserve_messages(
    queue_id: String,
    reserve_params: Json<ReserveMessageParams>,
    conn: RedisConnection
) -> Option<Json<Value>> {
    let rp = reserve_params.into_inner();
    let results: Vec<Message> = Queue::reserve_messages(&queue_id, &rp, &*conn);

    return Some(Json(json!({
        "messages": results
    })))
}

#[error(404)]
fn not_found() -> Json {
    Json(json!({
        "status": "error",
        "reason": "Resource was not found."
    }))
}
*/
pub fn main() {
    env_logger::init();

    info!("starting up");
    let pool = pool();
    let port: String = env::var("PORT").expect("$PORT is provided");
    info!("PORT: {:?}", port);

    let addr = format!("0.0.0.0:{}", port);
    info!("Gotham started on: {}", addr);
    gotham::start(addr, router(pool))
}
