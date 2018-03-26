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
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rayon;

mod middleware;
mod pool;
mod api;
mod mq;

use std::env;

use gotham::router::Router;
use gotham::router::builder::*;
use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;

use pool::*;
use middleware::redis::RedisMiddleware;

use api::queue::QueuePathExtractor;

fn router(pool: Pool) -> Router {
    let redis_middleware = RedisMiddleware::with_pool(pool);

    let (chain, pipelines) = single_pipeline(
        new_pipeline().add(redis_middleware).build()
    );

    build_router(chain, pipelines, |route| {
        route.get("/").to(api::index);

        route.scope("/redis", |route| {
            route.get("/version").to(api::redis::version);
        });

        route.get("/queues").to(api::queue::list_queues);

        route.scope("/queues/:name", |route| {
            route.put("")
                .with_path_extractor::<QueuePathExtractor>()
                .to(api::queue::put_queue);
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
        });
    })
}

pub fn main() {
    env_logger::init();

    info!("starting up");
    let pool = new_pool();
    let port: String = env::var("PORT").expect("$PORT is provided");
    info!("PORT: {:?}", port);

    let addr = format!("0.0.0.0:{}", port);
    info!("Gotham started on: {}", addr);
    gotham::start(addr, router(pool))
}
