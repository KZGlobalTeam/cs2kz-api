# CS2KZ API

This is the new API for [CS2KZ](https://github.com/KZGlobalTeam/cs2kz-metamod).

It is currently WIP and very likely to change! If you want to have a look around anyway, feel free.
If you want to run the API locally, please refer to [Dev Setup](#dev-setup).

> [!IMPORTANT]
> If you are a [CS2KZ](https://github.com/KZGlobalTeam/cs2kz-metamod) developer, see
> [KZ-DEVELOPERS.md](./KZ-DEVELOPERS.md).

# Dev Setup

The repository is made up of a few components:

- `src/` - the source code for the API
- `crates/` - any library / utility crates separate from the API
- `database/` - database related files, like migrations and docker volumes
- `workshop/` - any Steam Workshop artifacts downloaded by the API

You will want to install (some of) the following programs on your system:

- [Cargo](https://doc.rust-lang.org/cargo) - Rust's build tool. You will need the stable toolchain
  for compiling and running the API. Optionally, if you wish to contribute, please also install the
  nightly toolchain for `rustfmt` (the formatter).
- [Docker](https://www.docker.com) - for running a local database. While this is not strictly
  necessary, managing multiple databases that are installed directly on your
  system is painful. You can also run the API itself in a container if you want
  (this is how it is deployed).
- [just](https://github.com/casey/just) - for running commands. This is also not necessary; you
  can run everything manually if you wish to do so, but `just` makes it less annoying.
- [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli) - for running database
  migrations. This is also not strictly necessary; all migrations are `.sql` files stored in
  `./database/migrations` and you can run them against your database manually. The `justfile`
  however assumes that you're using `sqlx`, so some commands might not work if you don't have it
  installed.

Copy `.env.example` to `.env`, and `.env.docker.example` to
`.env.docker`. These files are used for configuration, and the API or Docker
might not work correctly if they are missing.

You can run `just` to see a list of useful commands. If you don't have `just`
installed, you can just look at the contents of the `justfile` to see what they
do.

To run the API, simply execute `cargo run` at the root of the repository. It
requires a running database, so make sure you set that up (I recommend Docker).

> [!IMPORTANT]
> Whenever you change database queries in the API code that are written using macros (i.e.
> `sqlx::query!` and `sqlx::query_as!`), you need to run `just sqlx-cache`. This ensures that the
> API will also compile without a live database, which is necessary for CI checks to pass.

# Contributions

If you wish to contribute to this repository, have a look at [CONTRIBUTING.md](./CONTRIBUTING.md)!
