FROM rustlang/rust:nightly

WORKDIR /usr/src/rusted-iron
COPY . .

RUN cargo build --release
#EXPOSE 80

RUN useradd -m iron
USER iron

CMD ROCKET_PORT=$PORT ROCKET_ENV=staging target/release/rusted-iron