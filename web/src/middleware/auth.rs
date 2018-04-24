use std::env;
use futures::{future, Future};
use hyper::{
    header::{
        Headers,
        Authorization,
        Basic,
        UserAgent
    },
    StatusCode,
};
use gotham::{
    http::response::create_response,
    handler::HandlerFuture,
    middleware::Middleware,
    state::{
        FromState,
        request_id,
        State
    }
};
use middleware::redis::RedisPool;
use auth::is_authenticated;

#[derive(StateData, Debug)]
pub struct AuthMiddlewareData {
    pub email: String,
    pub password_plain: String
}

#[derive(Clone, NewMiddleware)]
pub struct AuthMiddleware;

impl AuthMiddlewareData {
    pub fn from_header(header: &Authorization<Basic>) -> Self {
        let mut d = AuthMiddlewareData{
            email: String::new(),
            password_plain: String::new()
        };
        d.password_plain = match header.password.clone() {
            Some(v) => v,
            None => String::new(),
        };
        d.email = header.username.clone();

        d
    }
}

impl Middleware for AuthMiddleware {
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        let service_token: String = env::var("SERVICE_TOKEN").expect("$SERVICE_TOKEN is provided");
        let skip_auth = {
            let headers: &Headers = Headers::borrow_from(&state);
            match headers.get::<UserAgent>() {
                Some(header) => {
                    trace!("Header: {:#?}", header);
                    header.starts_with(service_token.as_str())
                },
                None => false,
            }
        };

        if skip_auth {
            let result = chain(state);

            let f = result.and_then(move |(state, response)| {
                {
                    trace!("[{}] post chain", request_id(&state));
                }

                future::ok((state, response))
            });

            Box::new(f)
        } else {
            let auth: AuthMiddlewareData = {
                let headers: &Headers = Headers::borrow_from(&state);

                match headers.get::<Authorization<Basic>>() {
                    Some(header) => AuthMiddlewareData::from_header(header),
                    None => {
                        ("No authorization header found");
                        AuthMiddlewareData{
                            email: String::new(),
                            password_plain: String::new()
                        }
                    }
                }
            };
            let connection = {
                let redis_pool = RedisPool::borrow_mut_from(&mut state);
                let connection = redis_pool.conn().unwrap();
                connection
            };

            let is_authenticated = is_authenticated(&auth, &connection);

            if !is_authenticated {
                let res = create_response(&state, StatusCode::Unauthorized, None);
                return Box::new(future::ok((state, res)))
            }

            state.put(auth);

            let result = chain(state);

            let f = result.and_then(move |(state, response)| {
                {
                    trace!("[{}] post chain", request_id(&state));
                }

                future::ok((state, response))
            });

            Box::new(f)
        }
    }
}
