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

    let v: Value= serde_json::from_str(&js).expect("my msg");
    let expected = json!({
        "goto": "http://www.iron.io"
    });

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    assert_eq!(v, expected);
}
