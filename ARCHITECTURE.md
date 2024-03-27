# The CS2KZ API Architecture

> This repository follows a certain structure, so if you want to understand it
> and/or contribute, keep reading!

> [!IMPORTANT]
> This is a [Rust](https://www.rust-lang.org) project, which means you should
> be familiar with [Cargo](https://doc.rust-lang.org/cargo), as it is the main
> build tool.

## Repository Structure

The repository is structured as a
[cargo workspace](https://doc.rust-lang.org/cargo/reference/workspaces.html),
with the root manifest at [./Cargo.toml](./Cargo.toml). Important files /
directories include:

* [`src`](./src) - the main API crate; this is where the bulk of the code is located
* [`crates`](./crates) - any utility libraries or helper crates (part of the workspace)
* [`database`](./database) - migrations, test fixtures, and docker volumes
* [`Justfile`](./Justfile) - command runner for developing
* `.env.*` files - environment variables that are loaded at runtime
   * you should copy both of them and remove the `.example` suffix
   * `.docker` is only used by the API while running inside a docker container
* [`docker-compose.yml`](./docker-compose.yml) - docker setup for the database & API
* [`Dockerfile`](./Dockerfile) - [Dockerfile](https://docs.docker.com/reference/dockerfile)
  for the API container

## `cs2kz-api` crate

The [`src`](./src) directory contains the API code. It is fairly complex, but
follows a specific structure.

`cs2kz-api` contains both a library and a binary crate. The binary's entry
point is [`main.rs`](./src/main.rs), which is only concerned with log capturing
and actually running the API.

[`lib.rs`](./src/lib.rs) is what contains all the logic.

* simple modules will be singular files, like [`state.rs`](./src/state.rs)
* "domains" all have their own modules, e.g. [`players`](./src/players) or
  [`maps`](./src/maps)
   * the entrypoint `mod.rs` exports a function called `router`, which describes
     the API routes covered by that module
   * every "domain" contains at least 1 module called `handlers`, which contains
     HTTP handlers for the corresponding routes
      * handler functions are named after their HTTP method, e.g. `get()`
      * handler functions are annotated with `#[utoipa::path]` macros for
        generating OpenAPI documentation
   * `models` and `queries` are used for shared types and SQL queries

### Core Libraries

The most important libraries you should be familiar with:

* [tokio](https://tokio.rs) - async runtime
* [axum](https://docs.rs/axum) - http framework
* [tracing](https://docs.rs/tracing) - logging
* [serde](https://docs.rs/serde) - (de)serializing e.g. JSON
* [utoipa](https://docs.rs/utoipa) - generating OpenAPI documentation

### Authentication

There are 3 main "sources" for authentication:

* CS2 Servers
* Steam
* Opaque API Keys

#### CS2 Servers

Every globally approved server has a "permanent" refresh key. This key is a UUID
that is randomly generated when the server is approved. It will only be exposed
once, and is not viewable again later. Admins and server owners can generate new
keys for their servers anytime they want, and will get to see that new key
exactly once as well. This key will then be put into a configuration file by the
server owner and used by the cs2kz plugin to request temporary JWTs. This is
done by making a `POST` request to `/servers/key` with the refresh key in the
request body. The returned JWT will be valid for 15 minutes and grants the
server access to protected endpoints (e.g. for submitting records). The refresh
key can be revoked by admins at any time, which will effectively "deglobal" a
server, because it won't be able to request new JWTs anymore.

#### Steam

Anyone with a Steam account can hit the `/auth/login` endpoint to login with
Steam. This will create a stateful session in the API's database and be queried
on every request. Users have permissions associated with them in the database,
so while auth**entication** is handled by Steam, auth**orization** is handled by
the API. Having a valid session cookie is necessary for making requests to
protected endpoints (e.g. for managing servers or bans). These sessions can be
invalidated using the `/auth/logout` endpoint, or by not making any requests for
7 consecutive days. Every request made will extend the current session to be
valid for the next 7 days.

#### Opaque API Keys

These are randomly generated UUIDs that don't have any special information
associated with them. They are used for automated tasks like submitting new
plugin versions via GitHub CD or by known services to bypass restrictions. They
have to be generated and revoked manually by admins.
