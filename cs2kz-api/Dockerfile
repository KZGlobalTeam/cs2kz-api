FROM lukemathwalker/cargo-chef:latest-rust-1.73.0 AS chef
WORKDIR /kz


# Compile and cache dependencies
FROM chef AS planner
COPY Cargo.toml .
COPY .sqlx .sqlx
COPY cs2kz cs2kz
COPY cs2kz-api cs2kz-api
RUN cargo chef prepare --recipe-path recipe.json


# Compile API crate
FROM chef AS builder
COPY --from=planner /kz/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml .
COPY .sqlx .sqlx
COPY cs2kz cs2kz
COPY cs2kz-api cs2kz-api
RUN cargo build --release --bin cs2kz-api


# Prepare runtime environment
FROM debian:bookworm-slim AS runtime
COPY configs/api.docker.toml /etc/cs2kz-api.toml
COPY --from=builder /kz/target/release/cs2kz-api /bin/cs2kz-api
ENV RUST_LOG=WARN,cs2kz_api=TRACE

# Launch API
ENTRYPOINT ["/bin/cs2kz-api", "--config", "/etc/cs2kz-api.toml"]
