[![Dependency Status](https://deps.rs/repo/github/kzglobalteam/cs2kz-api/status.svg)](https://deps.rs/repo/github/kzglobalteam/cs2kz-api)

# CS2KZ API

This project encapsulates the backend infrastructure of CS2KZ.
It is developed in tandem with [the plugin][cs2kz] and currently WIP.

If you want to run the API locally, see [Local Setup](#local-setup).
The recommended tooling for development is listed under [Tooling](#tooling).
The project structure is documented in [ARCHITECTURE.md](./ARCHITECTURE.md).

Questions and feedback are appreciated! Feel free to open an issue or join [our Discord][discord].

## Local Setup

> \[!IMPORTANT\]
> It is expected you have the required tools described in [tooling](#tooling) installed on your system.

The API uses a configuration file located in `.config/config.toml`. An example configuration file is provided in the
same directory with all the default values filled in, copy and modify it as you see fit. `.env.example` and
`.env.docker.example` should be copied to `.env` and `.env.docker` respectively. Again, change the default values as you
see fit.

The API requires a MariaDB instance in order to run. It is recommended that you run one using [Docker][] using the
`compose.yml` file provided by this repository. Install docker and run the following command:

```sh
docker compose up -d cs2kz-database
```

To compile the API itself, you can use `cargo`:

```sh
# also specify `--release` to enable optimizations
cargo build --locked --bin cs2kz-api serve

# compile & run in one step
cargo run --locked --bin cs2kz-api serve
```

To compile and run with Docker instead:

```sh
docker compose up --build cs2kz-api
```

The nix flake in the repository root also outputs the API binary as its default package.

### Testing

> \[!IMPORTANT\]
> Most of the tests in the `cs2kz-api` crate require a live database to run.

You can run the test suite for the whole workspace using:

```sh
cargo test --locked --workspace
```

### Debugging with [tokio-console][]

The API supports sending trace data to `tokio-console` so you can inspect the runtime in real time.
In order to use it, compile with the `console` feature enabled, and the `tokio_unstable` cfg flag in your `RUSTFLAGS`.

```sh
RUSTFLAGS="--cfg tokio_unstable" cargo run --locked --bin cs2kz-api --features console serve
```

The `debug` recipe in the `Justfile` does the same.

## Tooling

1. [rustup][] to install the Rust toolchain
2. [Docker][] for running the database and (optionally) the API itself
3. [sqlx-cli][] for managing database migrations
4. [DepotDownloader][] (optional) for downloading workshop maps; required for `PUT /maps` and `PATCH /maps/{map_id}`
5. [just][] (optional) as a command runner
6. [nix][] (optional) if you know you know

[cs2kz]: https://github.com/KZGlobalTeam/cs2kz-metamod
[discord]: https://www.discord.gg/csgokz
[Docker]: https://www.docker.com
[tokio-console]: https://docs.rs/tokio-console
[rustup]: https://rustup.rs
[sqlx-cli]: https://github.com/launchbadge/sqlx/tree/main/sqlx-cli
[DepotDownloader]: https://github.com/SteamRE/DepotDownloader
[just]: https://just.systems
[nix]: https://nixos.org

# Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md).
