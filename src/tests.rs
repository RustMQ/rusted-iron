use super::{rocket, main};
use rocket::local::*;
use rocket::http::{ContentType, Status};
use serde_json;
use serde_json::{Value, Error};

#[test]
fn index() {
    let rocket = super::rocket();
    let client = Client::new(rocket).expect("valid rocket instance");

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
    let rocket = super::rocket();
    let client = Client::new(rocket).expect("valid rocket instance");

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
    let rocket = super::rocket();
    let client = Client::new(rocket).expect("valid rocket instance");

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
    let rocket = super::rocket();
    let client = Client::new(rocket).expect("valid rocket instance");

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
    let rocket = super::rocket();
    let client = Client::new(rocket).expect("valid rocket instance");

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

#[test]
fn delete_message_by_id() {
    let rocket = super::rocket();
    let client = Client::new(rocket).expect("valid rocket instance");

    let mut response: LocalResponse = client.delete("/queue/1/messages/1")
        .header(ContentType::JSON)
        .dispatch();

    let body_str: String = response.body_string().unwrap();
    let v: Value = serde_json::from_str(&body_str).expect("response json");

    let expected = json!({
        "msg": "Deleted"
    });

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(v, expected);
}


#[test]
fn reserve_message() {
    let rocket = super::rocket();
    let client = Client::new(rocket).expect("valid rocket instance");

    let mut response: LocalResponse = client.post("/queue/1/reservations")
        .header(ContentType::JSON)
        .body(r#"{"n":1}"#)
        .dispatch();

    assert_eq!(response.status(), Status::Ok);
}
