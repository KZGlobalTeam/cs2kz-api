# CS2KZ API prototype

This is a prototype for how the new API could look like. Database setup included via docker, see
below for setup requirements.

## Requirements

> All of these except for Docker are included in the `flake.nix` dev-shell, if you use nix.

- [Docker](https://www.docker.com/) for the database (and the API if you don't want to run it locally)
- [Rust](https://www.rust-lang.org/) if you want to run the API locally without Docker.
- [make](https://www.gnu.org/software/make/) for running commands. If you are on Windows, look at
  the `Makefile` and run the commands yourself :tf:
- [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli) to run database migrations. The
  easiest way to install it is via `cargo`, so if you don't have that, you can use
  `./scripts/connect_to_db.sh < ./database/migrations/20231006182917_initial_schemas.up.sql` etc. to
  run the migrations manually.

## Setting everything up

You can bootstrap the database + migrations, as well as the API container by simply running `make`.
After everything has been built, you can use `make run` to run the API with Docker.

How to handle each one individually is described below.

## Managing the database

You can build the database container like so:

```sh
make db
```

You can run migrations using the `sqlx-cli`:

```sh
make migrations
```

- You can delete the data volume for the database by deleting `./database/volumes/cs2kz-database/`.
- You can connect to the database using `./scripts/connect_to_db.sh`.

## Running the API

If you want to run the API locally, you need to have [cargo](https://doc.rust-lang.org/stable/cargo/)
installed. You can build the crate like so:

```sh
make dev
```

If you don't have Rust installed and want to run the API inside of a Docker container instead, you
can do that like so:

```sh
make api
```
