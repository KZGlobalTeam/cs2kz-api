[![Dependency Status](https://deps.rs/repo/github/kzglobalteam/cs2kz-api/status.svg)](https://deps.rs/repo/github/kzglobalteam/cs2kz-api)

# CS2KZ API

This is the backend for CS2KZ that is responsible for storing records, maps,
servers, etc. and exposing them to the outside world. It is currently under
development together with
[cs2kz-metamod](https://github.com/KZGlobalTeam/cs2kz-metamod).

## Running the API

In order to run locally, you will have to install
[Docker](https://www.docker.com). This is the recommended way to run the
database, as well as the API itself if you don't have Rust installed. If you
have Rust installed (or want to install it:
[rustup](https://www.rust-lang.org/tools/install)), then you can run the API
itself outside of docker as well.

First, clone this repository:

```sh
$ git clone https://github.com/KZGlobalTeam/cs2kz-api
```

Then setup environment variables:

```sh
$ cp .env.example .env
$ cp .env.docker.example .env.docker
```

Now make sure the database is running:

```sh
$ docker compose up -d cs2kz-database
```

If you want to run the API in docker as well, run:

```sh
$ docker compose up cs2kz-api
```

If you want to run natively, you can use `cargo`:

```sh
$ cargo run
```

## Contributions

If you want to contribute, have a look at [CONTRIBUTING.md](./CONTRIBUTING.md)!
