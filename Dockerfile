FROM lukemathwalker/cargo-chef:latest-rust-1.76-slim-bullseye AS chef
WORKDIR /kz

FROM chef AS planner
COPY crates crates
COPY src src
COPY .sqlx .sqlx
COPY Cargo.toml Cargo.lock README.md .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as BUILDER
COPY --from=planner /kz/recipe.json recipe.json
RUN cargo chef cook --workspace --release --recipe-path recipe.json
COPY crates crates
COPY src src
COPY .sqlx .sqlx
COPY Cargo.toml Cargo.lock README.md .
COPY database/migrations database/migrations
RUN cargo build --release --features production

FROM debian:bullseye-slim AS runtime

ARG DEPOT_DOWNLOADER_URL

RUN apt-get update -y && apt-get install -y curl unzip libicu-dev
RUN curl -Lo downloader.zip $DEPOT_DOWNLOADER_URL
RUN unzip downloader.zip \
	&& rm downloader.zip \
	&& chmod +x DepotDownloader \
	&& mv DepotDownloader /bin/DepotDownloader

COPY --from=builder /kz/target/release/cs2kz-api /bin/cs2kz-api
COPY ./database/migrations ./database/migrations

ENTRYPOINT ["/bin/cs2kz-api"]
