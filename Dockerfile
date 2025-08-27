FROM rust:1.87-slim AS base

RUN apt update && apt install -y build-essential protobuf-compiler pkg-config libssl-dev

RUN cargo install --locked cargo-chef sccache

ENV RUSTC_WRAPPER=sccache SCCACHE_DIR=/sccache

FROM base AS planner

WORKDIR /app

COPY . .

RUN cargo chef prepare --recipe-path recipe.json

FROM base AS builder

WORKDIR /app

ENV CARGO_PROFILE_RELEASE_DEBUG=false
ENV CARGO_PROFILE_RELEASE_STRIP=symbols
ENV CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
ENV CARGO_PROFILE_RELEASE_LTO=true

COPY --from=planner /app/recipe.json recipe.json

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    set -e && \
    cargo build --release && \
    strip /app/target/release/Umem && \
    find /app/target/release -name "*.d" -delete && \
    rm -rf /app/target/release/build && \
    rm -rf /app/target/release/incremental && \
    rm -rf /app/target/release/deps/*.rlib 

FROM alpine:latest 

WORKDIR /usr/src/app

COPY --from=builder /app/target/release/Umem /bin/umem

EXPOSE 8080

CMD ["./bin/umem"]
