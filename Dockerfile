FROM rustlang/rust:nightly

ARG REDISCLOUD_URL

WORKDIR /usr/src/rusted-iron
COPY . .

ENV REDISCLOUD_URL ${REDISCLOUD_URL:-redis://redis:6379}

RUN cargo build --release
#EXPOSE 80

RUN useradd -m iron
USER iron

CMD [ "sh", "-c", "ROCKET_PORT=${PORT} ROCKET_ENV=staging target/release/rusted-iron" ]
