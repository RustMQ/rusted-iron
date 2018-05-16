use chrono::prelude::*;
use redis::{Connection, Commands};
use failure::Error;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    name: Option<String>,
    created_at: Option<DateTime<Utc>>
}

pub fn is_project_exists(project_id: String, connection: &Connection) -> Result<bool, Error> {
    let result: i32 = connection.sismember("projects", project_id)?;

    if result == 0 {
        Ok(false)
    } else if result == 1 {
        Ok(true)
    } else {
        panic!("No such key/member are found")
    }
}
