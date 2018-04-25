use serde_redis::RedisDeserialize;
use redis::*;
use failure::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>
}

impl User {
    pub fn find_by_email(email: String, con: &Connection) -> Result<User, Error> {
        let mut email_key = String::new();
        email_key.push_str("email:");
        email_key.push_str(email.as_str());

        let user_ids: Vec<String> = con.smembers(&email_key)?;
        if user_ids.is_empty() {
            return Err(format_err!("No user with key: {}", &email_key));
        }
        let user_id = user_ids.first();
        let mut user_key = String::new();
        user_key.push_str("user:");
        user_key.push_str(user_id.unwrap_or(&String::from("000000000000000000000000")).as_str());
        let v: Value = con.hgetall(&user_key)?;
        let user: User = v.deserialize()?;

        Ok(user)
    }
}
