FROM rust:latest as BUILDER

RUN apt update && apt install -y libssl-dev pkg-config
RUN update-ca-certificates

WORKDIR /usr/src/social

COPY . .

RUN cargo build --release
EXPOSE 8000
CMD [ "./target/release/social_auth" ]
