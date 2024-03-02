FROM lukemathwalker/cargo-chef:latest-rust-1.76-slim-bullseye AS chef
WORKDIR /kz

FROM chef AS planner
COPY Cargo.toml .
COPY Cargo.lock .
COPY crates crates
COPY src src
COPY .sqlx .sqlx
RUN cargo chef prepare --recipe-path recipe.json --bin api

FROM chef as BUILDER
COPY --from=planner /kz/recipe.json recipe.json
COPY crates crates
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml .
COPY Cargo.lock .
COPY src src
COPY .sqlx .sqlx
COPY database/migrations database/migrations
RUN cargo build --release --locked --bin api -F production

FROM debian:bullseye-slim AS runtime

ARG DEPOT_DOWNLOADER_URL
RUN apt-get update -y && apt-get install -y curl unzip libicu-dev
RUN curl -Lo downloader.zip $DEPOT_DOWNLOADER_URL
RUN unzip downloader.zip \
	&& rm downloader.zip \
	&& chmod +x DepotDownloader \
	&& mv DepotDownloader /bin/workshop_downloader

COPY docker-entrypoint.sh /docker-entrypoint.sh
COPY --from=builder /kz/target/release/api /bin/cs2kz-api
COPY ./database/migrations ./database/migrations

ENTRYPOINT ["/bin/cs2kz-api"]
