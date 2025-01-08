# Contributing

This document outlines the tooling used in this repository and how you can
contribute changes.

## Tooling

The following is a list of the most important tools you'll want to have
installed if you wish to work with this repository. [The Rust Toolchain] is
implied. You should be familiar with Rust.

### [MariaDB]

The API uses [MariaDB] as its backing database. This means that, if you want to
run the API locally, you will need a MariaDB instance as well. You can do this
however you like, but I recommend you use
[docker and `docker-compose`](#docker-and-docker-compose).

### [Docker] and [`docker-compose`]

Both the database and the API itself can be run in [Docker]. To make this easy,
you can use [`docker-compose`] with the [`compose.yaml`](./compose.yaml) file in
the repository root.

### [direnv]

Some parts of the project (the database in particular) require a `DATABASE_URL`
environment variable to be set both at compile-time and run-time. You can do
this manually, but I recommend you install [direnv] and inspect the `.envrc`
file. To set environment variables, create a `.env` file (which will be loaded
by `.envrc`):

```sh
$ cp .example.env .env
```

This way, any relevant variables will be set automatically whenever you enter
the repository.

[The Rust Toolchain]: https://www.rust-lang.org/tools/install
[MariaDB]: https://mariadb.org
[Docker]: https://www.docker.com
[`docker-compose`]: https://docs.docker.com/compose
[direnv]: https://direnv.net
