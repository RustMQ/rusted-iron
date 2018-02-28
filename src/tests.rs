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
    // TODO: get queue and check totalrecv counter
}

#[test]
fn push_message_with_two_messages() {
    let (rocket, conn) = super::rocket();
    let client = Client::new(rocket).expect("valid rocket instance");
    conn.expect("connection is valid");

    let mut response: LocalResponse = client.post("/queue/1/messages")
        .header(ContentType::JSON)
        .body(r#"[
	            {
		            "body": "Bla"
	            },
	            {
		            "body": "Bla2"
	            }
        ]"#)
        .dispatch();

    let body_str: String = response.body_string().unwrap();
    let v: Value = serde_json::from_str(&body_str).expect("response json");

    let expected = json!({
        "ids": [
            "1",
            "2"
        ],
        "msg": "Messages put on queue."
    });

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(v, expected);
    // TODO: get queue and check totalrecv counter
}

#[test]
fn get_message() {
    let (rocket, conn) = super::rocket();
    let client = Client::new(rocket).expect("valid rocket instance");
    conn.expect("connection is valid");

    let mut response: LocalResponse = client.get("/queue/1/messages/1")
        .header(ContentType::JSON)
        .dispatch();

    let body_str: String = response.body_string().unwrap();
    let v: Value = serde_json::from_str(&body_str).expect("response json");

    let expected = json!({
        "message": {
            "id": "1",
            "body": "Bla"
        }
    });

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(v, expected);
    // TODO: get queue and check totalrecv counter
}

#[test]
#[ignore]
fn get_message_not_found() {
    let (rocket, conn) = super::rocket();
    let client = Client::new(rocket).expect("valid rocket instance");
    conn.expect("connection is valid");

    let mut response: LocalResponse = client.get("/queue/1/messages/0")
        .header(ContentType::JSON)
        .dispatch();

    let body_str: String = response.body_string().unwrap();
    let v: Value = serde_json::from_str(&body_str).expect("response json");

    let expected = json!({
        "message": {
            "id": "1",
            "body": "Bla"
        }
    });

    assert_eq!(response.status(), Status::NotFound);
    // assert_eq!(v, expected);
    // TODO: get queue and check totalrecv counter
}
