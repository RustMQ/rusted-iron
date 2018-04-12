FROM rust:1.25 as build

WORKDIR /usr/src/rusted-iron-pusher

ADD Cargo.toml Cargo.toml

ADD src src

RUN cargo build --release

FROM heroku/heroku:16
RUN useradd -m iron
USER iron

WORKDIR /pusher
COPY --from=build /usr/src/rusted-iron-pusher/target/release/pusher .

CMD [ "/bin/bash", "-c", "env && ls -al && ./pusher" ]
