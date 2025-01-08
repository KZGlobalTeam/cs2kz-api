FROM rust:1.83-slim as builder

WORKDIR /kz
RUN rustup toolchain install nightly-2025-01-08

COPY Cargo.toml Cargo.lock .
COPY .example.env .example.env
COPY .cargo .cargo
COPY crates crates
COPY .sqlx .sqlx

RUN apt-get update -y && apt-get install -y pkg-config python3 python3.11-dev
ENV SQLX_OFFLINE 1
RUN cargo +nightly-2024-11-28 build --release --locked --package=cs2kz-api --bin=cs2kz-api

FROM debian:bookworm-slim AS runtime

ARG DEPOT_DOWNLOADER_URL

RUN apt-get update -y && apt-get install -y curl unzip libicu-dev pkg-config python3.11-dev
RUN curl -Lo downloader.zip "$DEPOT_DOWNLOADER_URL"
RUN unzip downloader.zip \
	&& rm downloader.zip \
	&& chmod +x DepotDownloader \
	&& mv DepotDownloader /usr/bin/DepotDownloader

COPY --from=builder /kz/target/release/cs2kz-api /bin/cs2kz-api

ENV RUST_LOG cs2kz=trace,sqlx=debug,warn

ENTRYPOINT ["/bin/cs2kz-api", "--ip", "0.0.0.0", "--config", "/etc/cs2kz-api.toml"]
