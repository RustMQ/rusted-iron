#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[get("/")]
fn index() -> &'static str {
    r#"{"goto":"http://www.iron.io"}"#
}

fn main() {
    rocket::ignite()
        .mount("/", routes![index])
        .launch();
}
