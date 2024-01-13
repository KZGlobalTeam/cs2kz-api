FROM lukemathwalker/cargo-chef:latest-rust-1.75-slim-bullseye AS chef
WORKDIR /kz

FROM chef AS planner
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

# Install SteamCMD
RUN apt-get update \
	&& apt-get install -y software-properties-common \
	&& apt-add-repository non-free \
	&& dpkg --add-architecture i386 \
	&& apt-get update \
	&& echo steam steam/question select "I AGREE" | debconf-set-selections \
	&& echo steam steam/license note "" | debconf-set-selections \
	&& apt-get install -y steamcmd \
	&& ln -s /usr/games/steamcmd /bin/steamcmd \
	&& steamcmd +quit

COPY --from=builder /kz/target/release/cs2kz-api /bin/cs2kz-api
COPY .env .env
COPY .env.docker .env.docker
RUN cat .env.docker >> .env

ENTRYPOINT ["/bin/cs2kz-api"]
