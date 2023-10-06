FROM lukemathwalker/cargo-chef:latest-rust-1.73.0 AS chef
WORKDIR /kz


FROM chef AS planner
COPY Cargo.toml Cargo.lock .
COPY configs configs
COPY cs2kz-api cs2kz-api
RUN cargo chef prepare --recipe-path recipe.json


FROM chef AS builder
COPY --from=planner /kz/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml Cargo.lock .
COPY configs configs
COPY cs2kz-api cs2kz-api
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin cs2kz-api


FROM debian:bookworm-slim AS runtime
COPY --from=builder /kz/configs/api.docker.toml /etc/cs2kz-api.toml
COPY --from=builder /kz/target/release/cs2kz-api /bin/cs2kz-api
ENV RUST_LOG=WARN,cs2kz_api=TRACE
ENTRYPOINT ["/bin/cs2kz-api", "--config", "/etc/cs2kz-api.toml"]
