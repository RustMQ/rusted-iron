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
use redis::{RedisError};
use db::{Conn};
use queue::{Queue};
use message::{Message};

#[get("/", format = "application/json")]
fn index() -> Json {
    Json(json!({
        "goto": "http://www.iron.io"
    }))
}

#[get("/version")]
fn redis_version(conn: db::Conn) -> Json {
    let info : redis::InfoDict = redis::cmd("INFO").query(&*conn).unwrap();
    let redis_version: String = info.get("redis_version").unwrap();

    Json(json!({
        "redis_version": redis_version
    }))
}

#[get("/<queue_id>", format = "application/json")]
fn get_queue_info(queue_id: String, conn: Conn) -> Json<Value> {
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
    conn: Conn
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
    conn: Conn
) -> Json<Value> {
    let m: Message = Queue::get_message(&queue_id, &message_id, &*conn).expect("Message return");

    return Json(json!({
        "message": m
    }))
}

#[error(404)]
fn not_found() -> Json {
    Json(json!({
        "status": "error",
        "reason": "Resource was not found."
    }))
}

fn rocket() -> (Rocket, Option<Conn>) {
    let pool = db::init_pool();
    println!("{:?}", pool);
    let conn = Some(Conn(pool.get().expect("database connection")));

    let rocket = rocket::ignite()
        .manage(pool)
        .mount("/", routes![index, static_files::all])
        .mount("/redis/", routes![redis_version])
        .mount("/queue/", routes![
            get_queue_info,
            post_message_to_queue,
            get_message_from_queue
        ])
        .catch(errors![not_found]);

    (rocket, conn)
}

fn main() {
    let (rocket, _conn) = rocket();
    rocket.launch();
}
