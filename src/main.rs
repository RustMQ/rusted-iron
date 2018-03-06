#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate redis;
extern crate r2d2;
extern crate r2d2_redis;
extern crate serde;
extern crate serde_json;
extern crate rand;
extern crate uuid;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;

mod static_files;
mod db;
mod queue;
mod message;
#[cfg(test)] mod tests;

use rocket::{Rocket};
use rocket_contrib::{Json, Value};
use db::{pool, RedisConnection};
use queue::{Queue};
use message::{Message, ReserveMessageParams};

#[get("/", format = "application/json")]
fn index() -> Json {
    Json(json!({
        "goto": "http://www.iron.io"
    }))
}

#[get("/version")]
fn redis_version(conn: RedisConnection) -> Json {
    let info : redis::InfoDict = redis::cmd("INFO").query(&*(conn.0)).unwrap();
    let redis_version: String = info.get("redis_version").unwrap();

    Json(json!({
        "redis_version": redis_version
    }))
}

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
    println!("Reserve params: {:?}", reserve_params);

    let rp = reserve_params.into_inner();
    println!("Reserve params: {:?}", rp);

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

fn rocket() -> Rocket {
    let rocket = rocket::ignite()
        .manage(pool())
        .mount("/", routes![index, static_files::all])
        .mount("/redis/", routes![redis_version])
        .mount("/queue/", routes![
            get_queue_info,
            post_message_to_queue,
            get_message_from_queue,
            delete_message_from_queue,
            reserve_messages
        ])
        .catch(errors![not_found]);

    rocket
}

fn main() {
    let rocket = rocket();
    rocket.launch();
}
