FROM rust:alpine AS build
COPY . /app
WORKDIR /app
RUN apk add --update musl-dev openssl-dev openssl-libs-static \
    && CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse \
    OPENSSL_NO_PKG_CONFIG=1 OPENSSL_STATIC=1 OPENSSL_DIR=/usr/ \
    cargo build -p typst-cli --release

FROM alpine:latest
WORKDIR /root/
COPY --from=build  /app/target/release/typst /bin
