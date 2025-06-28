FROM --platform=$BUILDPLATFORM tonistiigi/xx AS xx
FROM --platform=$BUILDPLATFORM rust:alpine AS build

COPY --from=xx / /

RUN apk add --no-cache clang lld
COPY . /app
WORKDIR /app
RUN --mount=type=cache,target=/root/.cargo/git/db \
    --mount=type=cache,target=/root/.cargo/registry/cache \
    --mount=type=cache,target=/root/.cargo/registry/index \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse \
    cargo fetch

ARG TARGETPLATFORM

RUN xx-apk add --no-cache musl-dev openssl-dev openssl-libs-static
RUN --mount=type=cache,target=/root/.cargo/git/db \
    --mount=type=cache,target=/root/.cargo/registry/cache \
    --mount=type=cache,target=/root/.cargo/registry/index \
    OPENSSL_NO_PKG_CONFIG=1 OPENSSL_STATIC=1 \
    OPENSSL_DIR=$(xx-info is-cross && echo /$(xx-info)/usr/ || echo /usr) \
    xx-cargo build -p typst-cli --release && \
    cp target/$(xx-cargo --print-target-triple)/release/typst target/release/typst && \
    xx-verify target/release/typst

FROM alpine:latest
ARG CREATED
ARG REVISION
LABEL org.opencontainers.image.authors="The Typst Project Developers <hello@typst.app>"
LABEL org.opencontainers.image.created=${CREATED}
LABEL org.opencontainers.image.description="A markup-based typesetting system"
LABEL org.opencontainers.image.documentation="https://typst.app/docs"
LABEL org.opencontainers.image.licenses="Apache-2.0"
LABEL org.opencontainers.image.revision=${REVISION}
LABEL org.opencontainers.image.source="https://github.com/typst/typst"
LABEL org.opencontainers.image.title="Typst Docker image"
LABEL org.opencontainers.image.url="https://typst.app"
LABEL org.opencontainers.image.vendor="Typst"

COPY --from=build  /app/target/release/typst /bin
# Create a non-root user for security
RUN adduser --system --no-create-home --shell /bin/false typst

# Switch to non-root user
USER typst

ENTRYPOINT [ "/bin/typst" ]
