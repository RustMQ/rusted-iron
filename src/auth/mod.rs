use bcrypt;
use bcrypt::{DEFAULT_COST, hash};
use redis::Connection;
use middleware::auth::AuthMiddlewareData;
use user::User;

pub fn encode(password: String) -> String {
    match hash(password.as_str(), DEFAULT_COST) {
        Ok(v) => v,
        Err(_e) => String::from("Failed to encode")
    }
}

pub fn verify(password: &str, encoded_value: &str) -> bool {
    match bcrypt::verify(password, encoded_value) {
        Ok(v) => v,
        Err(_e) => false
    }
}

pub fn is_authenticated(auth: &AuthMiddlewareData, con: &Connection) -> bool {
    let user: User = User::find_by_email(auth.email.clone(), con);

    let p = match user.password {
        Some(p) => p,
        None => String::new(),
    };

    verify(auth.password_plain.clone().as_str(), p.as_str())
}

#[cfg(test)]
mod tests {
    use auth::{encode, verify};

    #[test]
    fn can_verify_hash_generated() {
        let hash = "$2a$12$KJczViz/D69PAoDPN3Qszuc0cDNQdzYVzjujOVJ68bLH/5X/opfM.";
        assert!(verify("password", hash));
    }

    #[test]
    fn can_verify_own_generated_hash() {
        let encoded = encode("superuser123!".to_string());
        assert_eq!(true, verify("superuser123!", encoded.as_str()));
    }

    #[test]
    fn can_detect_wrong_password_with_own_generated_hash() {
        let encoded = encode("superuser123!".to_string());
        assert_eq!(false, verify("superuser1234!", &encoded));
    }
}
