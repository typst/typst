FROM nginx:latest

ARG CREATED
ARG REVISION

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

COPY nginx.conf /usr/share/nginx/default.conf
COPY dist/site /usr/share/nginx/html
