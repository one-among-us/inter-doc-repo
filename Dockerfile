FROM rust:alpine AS builder
RUN apk add --no-cache musl-dev gcc openssl-dev openssl-libs-static pkgconfig

WORKDIR /usr/src/app
COPY . .

RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:latest
RUN apk add --no-cache ca-certificates

WORKDIR /app
COPY --from=builder /usr/src/app/target/x86_64-unknown-linux-musl/release/oau-InterDocRep .

CMD ["./oau-InterDocRep"]
