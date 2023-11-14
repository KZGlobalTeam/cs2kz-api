# CS2KZ API

This is the new database and API for CS2KZ. Below you will find instructions on how to integrate
with it as a KZ plugin developer.

See [Dev Setup](#dev-setup) for instructions on how to set up a development environment to develop
the API.

# For KZ Devs

In addition to the public `GET` routes there are also a bunch of `POST` and `PUT` routes
specifically for the CS2KZ plugin which require authentication.

Every approved server will have an API Key associated with it.
This key acts as a refresh token to obtain temporary access tokens (JWT).
Every request that requires authentication should include that access-token as a header.
In addition to the JWT every request should also include these keys in their JSON request body:

```rust
struct Body {
    plugin_version: u16,
}
```

For detailed and standardized schemas have a look at the `api-spec.json` file at the root of the
repository. Alternatively the API also hosts a SwaggerUI web page at `/api/docs/swagger-ui`.

## Routes for CS2KZ servers

### GET `/auth/token`

This requests a new access token.

Access tokens expire after 30 minutes so every server should regularly request a new one.
The request headers should include the API Key in an `api-key` header.

The newly generated JWT will be sent back as a response that looks like this:

```rust
struct Response {
    token: String,
    expires_on: DateTime<Utc>,
}
```

Servers will be queried regularly via
[A2S](https://developer.valvesoftware.com/wiki/Server_queries#A2S_INFO) to make sure they are still
alive and reachable at the expected IP:port combination. If a server stays down for an extended
period of time, the server owner will be notified and the server will be invalidated.

### POST `/players`

This registers a new player.

Upon a player joining the server should fetch their data from this same endpoint but with the `GET`
method, and only if they get a 204 response send a `POST` request to this endpoint with the
following JSON body:

```rust
struct Body {
    /// The player's Steam name.
    name: String,

    /// The player's `SteamID`.
    steam_id: SteamID,

    /// The player's IP address.
    ip_address: Ipv4Addr,
}
```

### PUT `/players/{steam_id}`

A server should in regular intervals send updates about currently connected players to the API.
Such updates are `PUT` requests for specific players. They should include the following JSON body:

```rust
struct Body {
    /// The player's new name.
    name: Option<String>,

    /// The player's new IP address.
    ip_address: Option<Ipv4Addr>,
}
```

If neither of these have changed, an empty body should be sent.

### POST `/bans`

If the server-side Anti-Cheat or an admin decide that a player is cheating, the server sends
a `POST` request to this endpoint with the following JSON body:

```rust
struct Body {
    /// The player's `SteamID`.
    steam_id: SteamID,

    /// The player's IP address at the time of the ban.
    ip_address: Option<Ipv4Addr>,

    /// The reason for the ban.
    reason: BanReason,

    /// The `SteamID` of the admin who issued this ban.
    banned_by: Option<SteamID>,

    /// Timestamp of when this ban expires.
    expires_on: Option<DateTime<Utc>>,
}

// These are snake_case formatted as JSON
enum BanReason {
    AutoBhop,
}
```

### POST `/records`

Everytime a player finishes a course the server should send information about the run as well as
a replay to the API with the following JSON body:

```rust
struct Body {
    /// The ID of the course this record was performed on.
    course_id: u32,

    /// The mode this record was performed in.
    mode: Mode,

    /// The style this record was performed in.
    style: Style,

    /// The `SteamID` of the player who performed this record.
    steam_id: SteamID,

    /// The time it took to finish this run (in seconds).
    time: f64,

    /// The amount of teleports used in this run.
    teleports: u16,

    /// Statistics about how many perfect bhops the player hit during the run.
    bhop_stats: BhopStats,
}

enum Mode {
    Vanilla,
    Modded,
}

enum Style {
    Normal,
    Backwards,
    Sideways,
    WOnly,
}

struct BhopStats {
    perfs: u16,
    bhops_tick0: u16,
    bhops_tick1: u16,
    bhops_tick2: u16,
    bhops_tick3: u16,
    bhops_tick4: u16,
    bhops_tick5: u16,
    bhops_tick6: u16,
    bhops_tick7: u16,
    bhops_tick8: u16,
}
```

# Dev Setup

## System Requirements

You will need the following tools installed on your system:

- [Rust](https://rust-lang.org/) for compiling the API code
  - Stable toolchain for compiling
  - Nightly toolchain for formatting
- [Docker](https://www.docker.com/) for running a database and potentially the API
- [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli) for running database migrations
- [make](https://www.gnu.org/software/make/) for running commands (not required but useful)

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
