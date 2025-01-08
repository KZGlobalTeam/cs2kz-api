# CS2KZ API Architecture

This document gives a high-level overview over the architecture of this
repository and how to find your way around. It is **not** documentation about
what this project is or how it is used. For that, visit
<https://docs.cs2kz.org/api>.

## Crate Layout

All crates live in the `crates/` directory. Some are just utilites, like
`steam-id` and `problem-details`, but anything prefixed with `cs2kz-` is
specific to this project.

### `cs2kz` crate

This crate contains all the core business logic that the API performs.

### `cs2kz-api` crate

This crate contains the HTTP server and depends on `cs2kz`.

## Authentication & Authorization

The API provides two ways to authenticate requests:

1. [Sessions](#session-authentication)
2. [API Keys](#key-authentication)

### Session Authentication

Sessions are how other _applications_, such as websites, can authenticate with
the API. We use Steam as an OpenID provider to perform the actual login process,
and then store the user's SteamID alongside an opaque session ID in the
database. The session ID is given back to the user in a cookie, and they can use
it for future requests.

For authorization we primarily use a custom permission system. These are modeled
as bitflags, and every user has them. They are checked whenever a session is
fetched from the database, and can then be used to perform authorization. There
are other methods as well, all encapsulated in the `AuthorizeSession`
trait.

Check the `cs2kz_api::middleware::auth::session_auth` documentation and
implementation for details.

### Key Authentication

The API also stores a table of opaque keys that are used for one-off purposes,
such as GitHub actions, as well as keys for approved CS2 servers. CS2 servers
will use their key when establishing a WebSocket connection, which they will
then communicate over permanently.

Check the `cs2kz_api::middleware::auth::access_key` and `cs2kz_api::ws`
documentation and implementation for details.
