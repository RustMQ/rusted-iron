FROM rustlang/rust:nightly as build

WORKDIR /usr/src/rusted-iron
COPY . .

RUN cargo build --release

FROM heroku/heroku:16
RUN useradd -m iron
USER iron

COPY --from=build /usr/src/rusted-iron/target/release/rusted-iron .

CMD [ "/bin/bash", "-c", "env && ls -al && ROCKET_PORT=${PORT} ROCKET_ENV=staging ./rusted-iron" ]
