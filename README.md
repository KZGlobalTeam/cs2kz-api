# CS2KZ API

This is the new API for [CS2KZ](https://github.com/KZGlobalTeam/cs2kz-metamod).

It is currently WIP and very likely to change! If you want to have a look around anyway, feel free.
If you want to run the API locally, please refer to [Dev Setup](#dev-setup).

> [!IMPORTANT]
> If you are a [CS2KZ](https://github.com/KZGlobalTeam/cs2kz-metamod) developer, see
> [KZ-DEVELOPERS.md](./KZ-DEVELOPERS.md).

# Dev Setup

You will need to install the following programs on your system:

- [Cargo](https://doc.rust-lang.org/cargo) - Rust's build tool. You will need the stable toolchain
  for compiling and running the API. Optionally, if you wish to contribute, please also install the
  nightly toolchain for `rustfmt` (the formatter).
- [Docker](https://www.docker.com) - for running a local database. While this is not strictly
  necessary, it is highly recommended. Managing multiple databases that are installed directly on
  your system is a PITA. You can also run the API itself in a container if you want (this is how it
  is deployed).
- [Make](https://www.gnu.org/software/make) - for running commands. This is also not necessary; you
  can run everything manually if you wish to do so, but `make` makes it less annoying.
- [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli) - for running database
  migrations. This is also not strictly necessary; all migrations are `.sql` files stored in
  `./database/migrations` and you can run them against your database manually. The `Makefile`
  however assumes that you're using `sqlx`, so some commands might not work if you don't have it
  installed.

Next, all you need to do is run `make`. That should take care of everything! If not, the following
commands are useful:

```sh
# Create the database
$ make db

# Run migrations
$ make migrations

# Connect to the database
$ make db-connect

# Purge the database
$ make db-clean

# Run the API locally
$ make dev

# Run the API in a container
$ make api

# Lint your code
$ make check

# Format your code
$ make fmt

# Document your code
$ make docs
```

If you want to set any custom environment variables, copy `.env.example` to `.env` and set them in
there.

> [!IMPORTANT]
> Whenever you change database queries in the API code that are written using macros (i.e.
> `sqlx::query!` and `sqlx::query_as!`), you need to run `make sqlx-cache`. This ensures that the
> API will also compile without a live database, which is necessary for CI checks to pass.

# Contributions

If you wish to contribute to this repository, have a look at [CONTRIBUTING.md](./CONTRIBUTING.md)!
