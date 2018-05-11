extern crate futures;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate hyper;
extern crate mime;
extern crate redis;
extern crate r2d2;
extern crate r2d2_redis;
extern crate serde;
#[macro_use]
extern crate serde_json;
extern crate objectid;
#[macro_use]
extern crate serde_derive;
extern crate serde_redis;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rayon;
extern crate scheduled_thread_pool;
extern crate bcrypt;
extern crate queue;
extern crate chrono;
#[macro_use]
extern crate failure;

mod middleware;
mod pool;
mod api;
mod mq;
mod user;
mod auth;
mod project;

use std::env;

use gotham::{
    router::{
        Router,
        builder::*
    },
    pipeline::{
        new_pipeline,
        set::{finalize_pipeline_set, new_pipeline_set}
    }
};

use pool::*;
use middleware::{
    auth::{AuthMiddleware},
    redis::RedisMiddleware
};

use api::{
    queue::QueuePathExtractor,
    message::{
        QueryStringExtractor
    }
};

fn router(pool: Pool) -> Router {
    let redis_middleware = RedisMiddleware::with_pool(pool);
    let pipelines = new_pipeline_set();
    let (pipelines, default) = pipelines.add(
        new_pipeline()
            .add(redis_middleware.clone())
            .build()
    );

    let (pipelines, extended) = pipelines.add(
        new_pipeline()
            .add(redis_middleware.clone())
            .add(AuthMiddleware)
            .build()
    );

    let default_chain = (default, ());
    let extended_chain = (extended, default_chain);
    let pipeline_set = finalize_pipeline_set(pipelines);

    build_router(default_chain, pipeline_set, |route| {
        route.with_pipeline_chain((), |route| {
            route.get("/")
                .to(api::index);
        });
        route.scope("/redis", |route| {
            route.get("/version")
                .to(api::redis::version);
        });

        route.with_pipeline_chain(extended_chain, |route| {
            route.scope("/3/projects/:project_id", |route| {
                route.get("/queues")
                    .with_path_extractor::<QueuePathExtractor>()
                    .to(api::queue::list_queues);
                route.scope("/queues/:name", |route| {
                    route.put("")
                        .with_path_extractor::<QueuePathExtractor>()
                        .to(api::queue::put_queue);
                    route.patch("")
                        .with_path_extractor::<QueuePathExtractor>()
                        .to(api::queue::update_queue);
                    route.get("")
                        .with_path_extractor::<QueuePathExtractor>()
                        .to(api::queue::get_queue_info);
                    route
                        .post("/messages")
                        .with_path_extractor::<QueuePathExtractor>()
                        .to(api::queue::push_messages);
                    route
                        .post("/reservations")
                        .with_path_extractor::<QueuePathExtractor>()
                        .to(api::queue::reserve_messages);
                    route.delete("")
                        .with_path_extractor::<QueuePathExtractor>()
                        .to(api::queue::delete_queue);

                    route.delete("/messages/:message_id")
                        .with_path_extractor::<QueuePathExtractor>()
                        .to(api::message::delete);
                    route.delete("/messages")
                        .with_path_extractor::<QueuePathExtractor>()
                        .to(api::message::delete_messages);

                    route.post("/webhook")
                        .with_path_extractor::<QueuePathExtractor>()
                        .to(api::queue::push_messages_via_webhook);

                    route.get("/messages/:message_id")
                        .with_path_extractor::<QueuePathExtractor>()
                        .to(api::message::get_message);

                    route.post("/messages/:message_id/touch")
                        .with_path_extractor::<QueuePathExtractor>()
                        .to(api::message::touch_message);

                    route.get("/messages")
                        .with_path_extractor::<QueuePathExtractor>()
                        .with_query_string_extractor::<QueryStringExtractor>()
                        .to(api::message::peek_messages);

                    route.post("/messages/:message_id/release")
                        .with_path_extractor::<QueuePathExtractor>()
                        .to(api::message::release_message);
                    route.post("/subscribers")
                        .with_path_extractor::<QueuePathExtractor>()
                        .to(api::queue::update_subscribers);
                    route.put("/subscribers")
                        .with_path_extractor::<QueuePathExtractor>()
                        .to(api::queue::replace_subscribers);
                    route.delete("/subscribers")
                        .with_path_extractor::<QueuePathExtractor>()
                        .to(api::queue::delete_subscribers);

                    route.get("/messages/:message_id/subscribers")
                        .with_path_extractor::<QueuePathExtractor>()
                        .to(api::message::get_push_statuses);
                });
            });
        });
    })
}

pub fn main() {
    env_logger::init();

    let pool = new_pool();
    let port: String = env::var("PORT").expect("$PORT is provided");

    let addr = format!("0.0.0.0:{}", port);
    info!("Rusted-Iron web started on: {}", addr);
    gotham::start(addr, router(pool))
}
