FROM lukemathwalker/cargo-chef:latest-rust-1.75-slim-bullseye AS chef
WORKDIR /kz

FROM chef AS planner
RUN cargo install --locked --no-default-features --features mysql sqlx-cli
COPY Cargo.toml .
COPY crates crates
COPY src src
COPY .sqlx .sqlx
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as BUILDER
COPY --from=planner /kz/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml .
COPY Cargo.lock .
COPY crates crates
COPY src src
COPY .sqlx .sqlx
RUN cargo build --release --locked --package cs2kz-api

FROM debian:bullseye-slim AS runtime
WORKDIR /kz

ARG DEPOT_DOWNLOADER_URL
RUN apt-get update -y && apt-get install -y curl unzip libicu-dev
RUN curl -Lo downloader.zip $DEPOT_DOWNLOADER_URL
RUN unzip downloader.zip \
	&& rm downloader.zip \
	&& chmod +x DepotDownloader \
	&& mv DepotDownloader /bin/workshop_downloader

COPY docker-entrypoint.sh /docker-entrypoint.sh
COPY --from=planner /usr/local/cargo/bin/sqlx /bin/sqlx
COPY --from=builder /kz/target/release/cs2kz-api /bin/cs2kz-api
COPY ./database/migrations ./migrations

ENTRYPOINT ["/docker-entrypoint.sh"]
