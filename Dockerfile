FROM rustlang/rust:nightly as build

WORKDIR /usr/src/rusted-iron

ADD Cargo.toml Cargo.toml

ADD Cargo.lock Cargo.lock

ADD src src

RUN cargo update

RUN cargo build --release

FROM heroku/heroku:16
RUN useradd -m iron
USER iron

COPY --from=build /usr/src/rusted-iron/target/release/rusted-iron .

CMD [ "/bin/bash", "-c", "env && ls -al && ROCKET_PORT=${PORT} ROCKET_ENV=${ROCKET_ENV} ./rusted-iron" ]
