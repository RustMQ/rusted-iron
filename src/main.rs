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
use rocket_contrib::{Json};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct RedisInfo {
    version: String
}

#[get("/")]
fn index() -> String {
    String::from(r#"{"goto":"http://www.iron.io"}"#)
}

#[get("/version")]
fn redis_version(conn: db::Conn) -> Json {
    let info : redis::InfoDict = redis::cmd("INFO").query(&*conn).unwrap();
    let redis_info = RedisInfo {
        version: info.get("redis_version").unwrap()
    };

    Json(json!(redis_info))
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
        .mount("/redis/", routes![redis_version])
        .catch(errors![not_found]);

    (rocket, conn)
}

fn main() {
    rocket().0.launch();
}
