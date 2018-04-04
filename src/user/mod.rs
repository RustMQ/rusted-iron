use auth::{encode};

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
}
