FROM rust:alpine AS builder

COPY . /app
WORKDIR /app
RUN --mount=type=cache,target=/root/.cargo/git/db \
    --mount=type=cache,target=/root/.cargo/registry/cache \
    --mount=type=cache,target=/root/.cargo/registry/index \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse \
    cargo fetch

RUN cargo docit compile

FROM nginx:latest AS server
ARG CREATED
ARG REVISION

# Copy static docs site from builder step.
COPY --from=builder /app/docs/dist/site /usr/share/nginx/html

RUN cat > /usr/share/nginx/default.conf << 'EOF'
server {
    listen       80;
    listen  [::]:80;
    server_name  localhost;
    index index.html;
    root /usr/share/nginx/html;

    # redirect .html URLs to same page without html suffix.
    location ~ ^(.+)\.html$ {
        return 301 $1;
    }

    location / {
        try_files $uri $uri.html $uri/ =404;
    }

    # redirect server error pages to the static page /50x.html
    error_page   500 502 503 504  /50x.html;
    location = /50x.html {
        root   /usr/share/nginx/html;
    }
}
EOF

LABEL org.opencontainers.image.authors="The Typst Project Developers <hello@typst.app>"
LABEL org.opencontainers.image.created=${CREATED}
LABEL org.opencontainers.image.description="Static documentation for the Typst typesetting system"
LABEL org.opencontainers.image.documentation="https://typst.app/docs"
LABEL org.opencontainers.image.licenses="Apache-2.0"
LABEL org.opencontainers.image.revision=${REVISION}
LABEL org.opencontainers.image.source="https://github.com/typst/typst"
LABEL org.opencontainers.image.title="Typst Documentation Docker image"
LABEL org.opencontainers.image.url="https://typst.app"
LABEL org.opencontainers.image.vendor="Typst"
