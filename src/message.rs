#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    id: Option<i32>,
    body: String
}