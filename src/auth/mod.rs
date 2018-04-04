use bcrypt;
use bcrypt::{DEFAULT_COST, hash};

pub fn encode(password: String) -> String {
    match hash(password.as_str(), DEFAULT_COST) {
        Ok(v) => v,
        Err(_e) => String::from("Failed to encode")
    }
}

pub fn verify(password: String, encoded_value: String) -> bool {
    match bcrypt::verify(password.as_str(), encoded_value.as_str()) {
        Ok(v) => v,
        Err(_e) => false
    }
}

#[cfg(test)]
mod tests {
    use auth::{encode, verify};

    #[test]
    fn can_verify_hash_generated() {
        let hash = "$2a$12$KJczViz/D69PAoDPN3Qszuc0cDNQdzYVzjujOVJ68bLH/5X/opfM.";
        assert!(verify("password".to_string(), hash.to_string()));
    }

    #[test]
    fn can_verify_own_generated_hash() {
        let encoded = encode("superuser123!".to_string());
        assert_eq!(true, verify("superuser123!".to_string(), encoded));
    }

    #[test]
    fn can_detect_wrong_password_with_own_generated_hash() {
        let encoded = encode("superuser123!".to_string());
        assert_eq!(false, verify("superuser1234!".to_string(), encoded));
    }
}
