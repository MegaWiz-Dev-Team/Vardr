FROM rust:1.83-slim AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock* ./
COPY src/ src/
COPY static/ static/
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl docker.io && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/vardr /usr/local/bin/vardr
ENV PORT=9090
EXPOSE 9090
HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
    CMD curl -f http://localhost:9090/health || exit 1
CMD ["vardr"]
