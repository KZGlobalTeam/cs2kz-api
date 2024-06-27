set dotenv-load := true

default:
  @just --list

clippy *ARGS:
  cargo clippy --workspace --all-features --no-deps --tests {{ARGS}}

fmt *ARGS:
  cargo +nightly fmt --all {{ARGS}}

doc *ARGS:
  cargo doc --workspace --all-features --document-private-items {{ARGS}}
  cargo run --package spec-generator > api-spec.json

sqlx-cache *ARGS:
  cargo sqlx prepare --workspace {{ARGS}} -- --tests

test *ARGS:
  cargo test --workspace {{ARGS}} -- --nocapture

run-with-console *ARGS:
  RUSTFLAGS="--cfg tokio_unstable" cargo +nightly run --features console {{ARGS}}

create-database:
  docker compose up --detach --wait cs2kz-database

clean-database:
  docker compose down --timeout 1 cs2kz-database
  sudo rm -rf {{justfile_directory()}}/database/volumes/cs2kz

run-migrations *ARGS:
  sqlx migrate run --source {{justfile_directory()}}/database/migrations {{ARGS}}

precommit:
  just clippy
  just fmt
  just doc
  just sqlx-cache
