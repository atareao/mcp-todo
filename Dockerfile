# ── Etapa 1: Compilación ──────────────────────────────────────────────────────
FROM docker.io/library/rust:alpine3.23 AS builder

RUN apk add --no-cache --update \
            build-base \
            autoconf \
            gdb \
            musl-dev \
            pkgconfig \
            strace \
            openssl \
            openssl-dev \
            openssl-libs-static

WORKDIR /app

# Cache de dependencias (evita recompilar crates cada vez)
COPY Cargo.toml .
# RUN mkdir src && echo "fn main() {}" > src/main.rs && \
#     cargo build --release && \
#     rm -rf src

# Compilación real
COPY src/ ./src/
RUN cargo build --release && \
    strip target/release/mcp-todo

# ── Etapa 2: Runtime ──────────────────────────────────────────────────────────
FROM docker.io/library/alpine:3.23

RUN apk add --update --no-cache \
    ca-certificates \
    curl \
    openssl \
    && \
    adduser -S -u 1000 -D mcp

COPY --from=builder /app/target/release/mcp-todo /usr/local/bin/

USER mcp
EXPOSE 3003

CMD ["mcp-todo"]
