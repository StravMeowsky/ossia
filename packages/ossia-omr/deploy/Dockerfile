FROM rust:1.91-bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake clang libclang-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY packages/ ./packages/
RUN cargo build --release -p ossia-omr --locked
