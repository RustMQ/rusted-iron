extern crate mime;

pub mod queue;
pub mod redis;

use hyper::{Response, StatusCode};
use gotham::state::{State};
use gotham::http::response::create_response;

pub fn index(state: State) -> (State, Response) {
    let res = {
        let res_str = r#"{
            "goto": "http://www.iron.io"
        }"#;

        create_response(
            &state,
            StatusCode::Ok,
            Some((
                res_str.to_string().into_bytes(),
                mime::APPLICATION_JSON
            )),
        )
    };

    (state, res)
}
