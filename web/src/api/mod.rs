extern crate mime;
extern crate serde_json;

pub mod redis;
pub mod queue;
pub mod message;

use hyper::{Response, StatusCode};
use gotham::{
    handler::IntoResponse,
    http::response::create_response,
    state::{
        State
    }
};

#[derive(Serialize)]
pub struct Index {
    goto: String,
}

impl IntoResponse for Index {
    fn into_response(self, state: &State) -> Response {
        create_response(
            state,
            StatusCode::Ok,
            Some((
                serde_json::to_string(&self).expect("serialized index").into_bytes(),
                mime::APPLICATION_JSON
            ))
        )
    }
}

pub fn index(state: State) -> (State, Index) {
    let index = Index {
        goto: "http://www.iron.io".to_string(),
    };

    (state, index)
}
