use super::{rocket, main};
use rocket::local::*;
use rocket::http::{ContentType, Status};
use db::Conn;
use serde_json;
use serde_json::{Value, Error};

#[test]
fn index() {
    let (rocket, conn) = super::rocket();
    let client = Client::new(rocket).expect("valid rocket instance");
    conn.expect("connection is valid");
    let mut response: LocalResponse = client.get("/").dispatch();
    let js: String = response.body_string().unwrap();

    let v: Value= serde_json::from_str(&js).expect("response json");
    let expected = json!({
        "goto": "http://www.iron.io"
    });

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    assert_eq!(v, expected);
}

#[test]
fn push_message_with_empty_list() {
let (rocket, conn) = super::rocket();
    let client = Client::new(rocket).expect("valid rocket instance");
    conn.expect("connection is valid");

    let mut response: LocalResponse = client.post("/queue/1/messages")
        .header(ContentType::JSON)
        .body(r#"[]"#)
        .dispatch();

    let body_str: String = response.body_string().unwrap();
    let v: Value = serde_json::from_str(&body_str).expect("response json");

    let expected = json!({
        "ids": [],
        "msg": "Messages put on queue."
    });

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(v, expected);
}