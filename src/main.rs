#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate redis;
extern crate r2d2;
extern crate r2d2_redis;

mod db;

use std::io::Cursor;
use r2d2_redis::RedisConnectionManager;
use rocket::*;
use rocket::http::*;
use rocket::response::*;

#[get("/")]
fn index() -> String {
    String::from(r#"{"goto":"http://www.iron.io"}"#)
}

#[get("/version")]
fn redis_version(conn: db::Conn) -> String {
    let info : redis::InfoDict = redis::cmd("INFO").query(&*conn).unwrap();
    let version : Option<String> = info.get("redis_version");

    format!("Redis version: {} ", version.unwrap())
}

fn rocket() -> (Rocket, Option<db::Conn>) {
    let pool = db::init_pool();
    let conn = Some(db::Conn(pool.get().expect("database connection")));

    let rocket = rocket::ignite()
        .manage(pool)
        .mount("/", routes![index])
        .mount("/redis/", routes![redis_version]);

    (rocket, conn)
}

fn main() {
    rocket().0.launch();
}
