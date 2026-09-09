FROM rustlang/rust:nightly-bookworm AS chef

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libglib2.0-dev \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    libva-dev \
    libdrm-dev \
    libx11-dev \
    libxext-dev \
    libxfixes-dev \
    libxdamage-dev \
    build-essential \
    g++ \
    clang \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef

WORKDIR /rift

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /rift/recipe.json recipe.json
COPY Cargo.toml Cargo.toml

ENV OPENSSL_STATIC=yes
ENV OPENSSL_VENDORED=yes
ENV CC=clang
ENV CXX=clang++
ENV SQLX_OFFLINE=true

RUN cargo chef cook --release --bin notification-service

COPY . .
RUN cargo build --release --bin notification-service

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libglib2.0-0 \
    libgstreamer1.0-0 \
    libva2 \
    libdrm2 \
    libgcc-s1 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home --shell /usr/sbin/nologin rift

COPY --from=builder /rift/target/release/notification-service /usr/local/bin/notification-service
USER 10001:10001
ENTRYPOINT ["notification-service"]
