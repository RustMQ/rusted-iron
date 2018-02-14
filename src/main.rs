#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate redis;
extern crate r2d2;
extern crate r2d2_redis;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;

mod db;

use rocket::{Rocket};
use rocket::http::{RawStr};
use rocket_contrib::{Json};
use redis::RedisError;

#[get("/")]
fn index() -> String {
    String::from(r#"{"goto":"http://www.iron.io"}"#)
}

#[get("/version")]
fn redis_version(conn: db::Conn) -> Json {
    let info : redis::InfoDict = redis::cmd("INFO").query(&*conn).unwrap();
    let redis_version: String = info.get("redis_version").unwrap();

    Json(json!({
        "redis_version": redis_version
    }))
}

#[get("/message/<key>")]
fn redis_get_key(key: &RawStr, conn: db::Conn) -> Json {
    let key_as_str = key.as_str();
    let result: Result<String, RedisError> = redis::Cmd::new().arg("GET").arg(key_as_str).query(&*conn);
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
        .mount("/redis/", routes![redis_version, redis_get_key])
        .catch(errors![not_found]);

    (rocket, conn)
}

fn main() {
    rocket().0.launch();
}
