# The CS2KZ API Architecture

This document aims to explain how this project is structured, which libraries you should be familiar with, and how the
API works at a high level. If you're new to the project, this is the right place to start!

> [!IMPORTANT]
> As this is a [Rust](https://www.rust-lang.org) project, you should be familiar with
> [Cargo](https://doc.rust-lang.org/cargo), its main build tool and package manager.
> It is expected that you have a general understanding of how Rust projects are structured and how the module system
> works.

## Repository Structure

This project is organized into several crates within a [Cargo workspace][workspace]. The next few sections will focus on
the most important ones. The main crate, [`cs2kz-api`](#cs2kz-api-crate), lives in [`./src/`](./src/) and declares its
metadata inside [`Cargo.toml`](./Cargo.toml). [`./lib/`](./lib/) contains helper libraries, most notably
[`cs2kz`](#cs2kz-crate).

- The [`./database/`](./database/) directory contains database migrations, test fixtures, and docker volumes.
- The [`./nix/`](./nix/) directory contains `.nix` files referenced by [`flake.nix`](./flake.nix).
- The `./logs/` directory will be created when using the default `LOG_DIR` configuration value as specified in
  [`.env.example`](./.env.example), and stores log files created by the API.
- The `./workshop/` directory will be created when using the default `KZ_API_WORKSHOP_PATH` configuration value as
  specified in [`.env.example`](./.env.example), and stores downloaded Steam Workshop files.
- The `./docker/` directory will contain directories for mounted volumes used by the API container.

### `cs2kz` crate

The "standard library" of CS2KZ.

It mosty contains type definitions for core concepts such as `SteamID` and `Mode`, and is used by most other crates in
the workspace.

### `cs2kz-api-macros` crate

A companion crate for `cs2kz-api`, containing procedural macros.

Currently it is necessary to define procedural macros in their own crates with a special `proc-macro = true` flag in
their `Cargo.toml`. Any macros that were written specifically for this project live in that crate.

## Services

[`tower::Service`][tower-service] is the core abstraction that `axum` builds on to handle requests. As such, they are an
important concept to understand. You'll find that `cs2kz-api` exports a module called `services`. This module contains
types which handle different parts of the system/domain, such as the `PlayerService` or the `MapService`. These usually
map directly to HTTP routes, such as `/players` or `/maps`. Some services don't, and are instead used by other services,
like `JwtService`. It's important to note that these types do not actually implement the `tower::Service` trait. As they
are application code, and not used in generic contexts, it would make little sense to actually implement those traits
for them. They follow the general structure of `async fn(Request) -> Response` by exposing public functions taking
a single `req` parameter and returning some response type. They are only concerned with business logic and don't know
anything about HTTP. The HTTP handlers for each service live in their own module, usually `http.rs`, and just call into
the service. Each service is then passed as router state using [`Router::with_state()`][axum-router-state] and extracted
in the handler functions. The request/response types are defined in `models.rs` and also publicly exported.

They _do_ know about database queries; there is no "repository" abstraction. This might change in the future, but
currently 99% of the "business logic" consists of database queries. I don't think there is a good reason to abstract
this away further, as it would just needlessly complicate things.

## Authentication & Authorization

The API provides several ways to authenticate requests:

1. [Sessions](#session-authentication)
2. [JWTs](#jwt-authentication)
3. [API Keys](#key-authentication)

### Session Authentication

Sessions are how other _applications_, such as websites, can authenticate with the API. We use Steam as an OpenID
provider to perform the actual login process, and then store the user's SteamID alongside an opaque session ID in the
database. The session ID is given back to the user in a cookie, and they can use it for future requests.

For authorization we primarily use a custom permission system. These are modeled as bitflags, and every user has
them. They are checked whenever a session is fetched from the database, and can then be used to perform
authorization. There are other methods as well, all encapsulated in the `AuthorizeSession` trait. Check the
`AuthService` documentation and implementation for details.

### JWT Authentication

JWTs are how CS2 servers authenticate with the API. Every server has a permanent refresh key, which they can use to
obtain temporary access tokens (JWTs). These access tokens are short-lived and are used to authenticate all requests.
Server owners receive their refresh key when their server is approved, and can reset it at any time. Global Admins may
_delete_ a server's refresh key, preventing it from generating new access tokens. This is usually done if a server
breaks the rules, and server owners are informed when it happens.

### Key Authentication

The API also stores a table of opaque keys that are used for one-off purposes, such as GitHub actions. These are
supposed to be used for internal processes, and aren't given out to random people.

[workspace]: https://doc.rust-lang.org/cargo/reference/workspaces.html
[future]: https://doc.rust-lang.org/std/future/index.html
[tokio-docs]: https://docs.rs/tokio/1
[axum-router]: https://docs.rs/axum/0.7/axum/struct.Router.html
[axum-query]: https://docs.rs/axum/0.7/axum/extract/struct.Query.html
[axum-json]: https://docs.rs/axum/0.7/axum/struct.Json.html
[axum-router-state]: https://docs.rs/axum/0.7/axum/struct.Router.html#method.with_state
[tower-service]: https://docs.rs/tower/0.4/tower/trait.Service.html
[sqlx-database]: https://docs.rs/sqlx/0.8/sqlx/trait.Database.html
[sqlx-encode]: https://docs.rs/sqlx/0.8/sqlx/trait.Encode.html
[sqlx-decode]: https://docs.rs/sqlx/0.8/sqlx/trait.Decode.html
[sqlx-from-row]: https://docs.rs/sqlx/0.8/sqlx/trait.FromRow.html
[sqlx-query-builder]: https://docs.rs/sqlx/0.8.0/sqlx/struct.QueryBuilder.html
[tracing-docs]: https://docs.rs/tracing/0.1
[tracing-subscriber]: https://docs.rs/tracing/0.1/tracing/trait.Subscriber.html
