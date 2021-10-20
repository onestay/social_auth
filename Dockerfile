FROM rust:latest as BUILDER

RUN apt update && apt install -y libssl-dev pkg-config
RUN update-ca-certificates

WORKDIR /usr/src/social

COPY . .

RUN cargo build --release

CMD [ "./target/release/social_auth" ]