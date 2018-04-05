use std::collections::HashMap;
use auth::{encode};
use redis::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>
}

impl User {
    pub fn new(first_name: String, last_name: String, email: String, password: String) -> Result<User, String> {
        let encoded_password = encode(password);
        if encoded_password == String::from("Failed to encode") {
            return Err("not created".to_string())
        }

        return Ok(
            User {
                first_name: Some(first_name),
                last_name: Some(last_name),
                email: Some(email),
                password: Some(encoded_password)
            }
        )
    }

    pub fn new_from_hash(map: HashMap<String, String>) -> Self {
        let mut user = User{
            first_name: None,
            last_name: None,
            email: None,
            password: None
        };

        match map.get(&*"first_name") {
            Some(v) => {
                user.first_name = Some(v.to_string());
            },
            _ => user.first_name = None
        }

        match map.get(&*"last_name") {
            Some(v) => {
                user.last_name = Some(v.to_string());
            },
            _ => user.last_name = None
        }

        match map.get(&*"email") {
            Some(v) => {
                user.email = Some(v.to_string());
            },
            _ => user.email = None
        }

        match map.get(&*"password") {
            Some(v) => {
                user.password = Some(v.to_string());
            },
            _ => user.password = None
        }

        user
    }

    pub fn find_by_email(email: String, con: &Connection) -> Self {
        let mut email_key = String::new();
        email_key.push_str("email:");
        email_key.push_str(email.as_str());

        let user_ids: Vec<String> = con.smembers(email_key).unwrap();
        let mut user_key = String::new();
        user_key.push_str("user:");
        user_key.push_str(user_ids.first().unwrap().as_str());
        let user_map: HashMap<String, String> = con.hgetall(user_key).unwrap();

        User::new_from_hash(user_map)
    }
}
