FROM rust:alpine AS build
COPY . /app
WORKDIR /app
RUN apk add --update git musl-dev \
    && cargo build -p typst-cli --release

FROM alpine:latest  
WORKDIR /root/
COPY --from=build  /app/target/release/typst /bin
