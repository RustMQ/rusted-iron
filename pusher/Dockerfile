FROM rust:1.25 as build

WORKDIR /usr/src/queue

ADD ./queue/Cargo.toml Cargo.toml

ADD ./queue/src src

RUN cargo build --release

WORKDIR /usr/src/pusher

ADD ./pusher/Cargo.toml Cargo.toml

ADD ./pusher/src src

RUN cargo build --release

FROM rust:1.25
RUN useradd -m iron
USER iron

WORKDIR /pusher
COPY --from=build /usr/src/pusher/target/release/pusher .

CMD [ "/bin/bash", "-c", "env && ls -al && ./pusher" ]
