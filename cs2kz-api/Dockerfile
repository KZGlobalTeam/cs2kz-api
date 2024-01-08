FROM lukemathwalker/cargo-chef:latest-rust-1.75-slim-bullseye AS chef
WORKDIR /kz

FROM chef AS planner
COPY Cargo.toml .
COPY cs2kz cs2kz
COPY macros macros
COPY spec-generator spec-generator
COPY cs2kz-api cs2kz-api
COPY .sqlx .sqlx
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as BUILDER
COPY --from=planner /kz/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml .
COPY cs2kz cs2kz
COPY macros macros
COPY spec-generator spec-generator
COPY cs2kz-api cs2kz-api
COPY .sqlx .sqlx
RUN cargo build --release --package cs2kz-api

FROM cm2network/steamcmd AS runtime
RUN mkdir /home/steam/downloads
COPY --from=builder /kz/target/release/cs2kz-api /bin/cs2kz-api
COPY .env .env
COPY .env.docker .env.docker
USER root
RUN chown steam:steam .env .env.docker
USER steam
RUN cat .env.docker >> .env

ENTRYPOINT ["/bin/cs2kz-api"]
