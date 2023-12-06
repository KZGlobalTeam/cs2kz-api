# CS2KZ API

This is the new database and API for CS2KZ.

It is currently WIP and there are still open questions about the design. For now though, feel free
to have a look around! If you want to run the API locally, see [Dev Setup](#dev-setup).

# Dev Setup

## System Requirements

You will need the following tools installed on your system:

- [Rust](https://rust-lang.org/) for compiling the API code
  - Stable toolchain for compiling
  - Nightly toolchain for formatting
- [Docker](https://www.docker.com/) for running a database and potentially the API
- [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli) for running database migrations
  (you can also run them manually by running the files in `./database/migrations` against your
  database directly, but `sqlx` is easier)
- [make](https://www.gnu.org/software/make) for running commands (not required but useful; see
  `Makefile`)

## Setting up the database

The database is managed by docker and while you could set it up manually directly on your system,
docker is highly recommended.

There is a `docker-compose.yml` file at the root of the repository which includes the container
definitions for both the database and the API.

You can spin up a database container by running either of the following commands:

```sh
$ make db
```

```sh
$ docker compose up -d cs2kz-database
```

When the database is up and running you will want to run migrations. You can do that by running
either of these commands:

```sh
$ make migrations
```

```sh
$ sqlx migrate run \
  --source ./database/migrations/ \
  --database-url mysql://kz:csgo-kz-is-dead-boys@127.0.0.1:8070/cs2kz-api
```

All the data in the database is persistent and you can delete it by getting rid of the
`./database/volumes/cs2kz-database/` directory.

You can connect to the database using the `DATABASE_URL` provided in `.env.example`.

## Building and running the API

If you have Rust installed you can compile and run the API with a single command:

```sh
$ cargo run -p cs2kz-api
```

If you wish to use Docker instead you can run either of these commands:

```sh
$ make api && make run
```

```sh
$ docker compose up cs2kz-api
```
