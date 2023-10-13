FROM rust:alpine AS build
COPY . /app
WORKDIR /app
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN apk add --update musl-dev \
    && cargo build -p typst-cli --release

FROM alpine:latest
WORKDIR /root/
COPY --from=build  /app/target/release/typst /bin
