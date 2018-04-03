use bcrypt::{DEFAULT_COST, hash, verify};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>
}

impl User {
    pub fn new(first_name: String, last_name: String, email: String, password: String) -> Result<User, String> {
        let encoded_password = User::encode(password);
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

    pub fn encode(password: String) -> String {
        match hash(password.as_str(), DEFAULT_COST) {
            Ok(v) => v,
            Err(_e) => String::from("Failed to encode")
        }
    }

    pub fn verify(password: String, encoded_value: String) -> bool {
        match verify(password.as_str(), encoded_value.as_str()) {
            Ok(v) => v,
            Err(_e) => false
        }
    }
}

#[cfg(test)]
mod tests {
    use user::User;

    #[test]
    fn can_verify_hash_generated() {
        let hash = "$2a$12$KJczViz/D69PAoDPN3Qszuc0cDNQdzYVzjujOVJ68bLH/5X/opfM.";
        assert!(User::verify("password".to_string(), hash.to_string()));
    }

    #[test]
    fn can_verify_own_generated_hash() {
        let encoded = User::encode("superuser123!".to_string());
        assert_eq!(true, User::verify("superuser123!".to_string(), encoded));
    }

    #[test]
    fn can_detect_wrong_password_with_own_generated_hash() {
        let encoded = User::encode("superuser123!".to_string());
        assert_eq!(false, User::verify("superuser1234!".to_string(), encoded));
    }
}