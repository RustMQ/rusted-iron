FROM rust:latest as build

WORKDIR /usr/src/rusted-iron

ADD Cargo.toml Cargo.toml

ADD Cargo.lock Cargo.lock

ADD src src
ADD static static

RUN cargo update

RUN cargo build --release

FROM heroku/heroku:16
RUN useradd -m iron
USER iron

WORKDIR /app
COPY --from=build /usr/src/rusted-iron/target/release/rusted-iron .
COPY --from=build /usr/src/rusted-iron/static static

CMD [ "/bin/bash", "-c", "env && ls -al && ./rusted-iron" ]
