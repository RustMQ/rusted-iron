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

mod db;
mod message;

use rocket::{Rocket};
use rocket::http::{RawStr};
use rocket_contrib::{Json, Value};
use redis::RedisError;
use rand::{thread_rng, Rng};
use uuid::Uuid;
use message::{Message};

#[get("/")]
fn index() -> String {
    String::from(r#"{"goto":"http://www.iron.io"}"#)
}

const DEFAULT_QUEUE: &'static str = "queue:Q";

#[get("/version")]
fn redis_version(conn: db::Conn) -> Json {
    let info : redis::InfoDict = redis::cmd("INFO").query(&*conn).unwrap();
    let redis_version: String = info.get("redis_version").unwrap();

    Json(json!({
        "redis_version": redis_version
    }))
}

#[get("/message/<key>")]
fn redis_get_key(key: &RawStr, conn: db::Conn) -> Json<Value> {
    let key_as_str = key.as_str();
    let result: Result<String, RedisError> = redis::Cmd::new().arg("HGET").arg(DEFAULT_QUEUE).arg(key_as_str).query(&*conn);
    match result {
        Ok(v) => {
            return Json(json!({
                "key": key_as_str.to_string(),
                "value": String::from(v)
            }))
        },
        Err(e) => {
            return Json(json!({
                "status": "error",
                "reason": format!("Internal Server Error: {}", e)
            }))
        },
    }
}

#[post("/message", format = "application/json", data = "<message>")]
fn redis_new_message(message: Json<Message>, conn: db::Conn) -> Json<Value> {
    println!("{:?}", message);
    let s: String = thread_rng().gen_ascii_chars().take(10).collect();
    println!("{}", s);
    let totalrecv: u64 = redis::Cmd::new().arg("HGET").arg(DEFAULT_QUEUE).arg("totalrecv").query(&*conn).unwrap();
    println!("Total message in queue: {}", totalrecv );
    let mid = Uuid::new_v5(&uuid::NAMESPACE_DNS, &s);
    println!("{}", mid);
    let result: Result<Vec<u64>, RedisError> = redis::pipe()
        .cmd("zadd").arg("queue").arg(totalrecv).arg(s.clone()).ignore()
        .cmd("hset").arg(DEFAULT_QUEUE).arg(s.clone()).arg(mid.hyphenated().to_string()).ignore()
        .cmd("hincrby").arg(DEFAULT_QUEUE).arg("totalrecv").arg(1).query(&*conn);

    match result {
        Ok(v) => {
            println!("Messge: {:?}", v);
            return Json(json!({
                "mid": s.clone(),
                "status": "ok"
            }))
        },
        Err(e) => {
            return Json(json!({
                "status": "error",
                "reason": format!("Internal Server Error: {}", e)
            }))
        },
    }
}

#[error(404)]
fn not_found() -> Json {
    Json(json!({
        "status": "error",
        "reason": "Resource was not found."
    }))
}

fn rocket() -> (Rocket, Option<db::Conn>) {
    let pool = db::init_pool();
    println!("{:?}", pool);
    let conn = Some(db::Conn(pool.get().expect("database connection")));

    let rocket = rocket::ignite()
        .manage(pool)
        .mount("/", routes![index])
        .mount("/redis/", routes![redis_version, redis_get_key, redis_new_message])
        .catch(errors![not_found]);

    (rocket, conn)
}

fn main() {
    rocket().0.launch();
}
