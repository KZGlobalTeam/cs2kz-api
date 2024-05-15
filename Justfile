set dotenv-load := true

default:
	@just --list

build *ARGS:
	cargo build --workspace {{ARGS}}

clippy *ARGS:
	cargo clippy --workspace --all-features --tests {{ARGS}}

fmt *ARGS:
	cargo +nightly fmt --all {{ARGS}}

doc *ARGS:
	cargo doc --workspace --all-features --document-private-items {{ARGS}}
	cargo run --package spec-generator > api-spec.json

sqlx-cache *ARGS:
	cargo sqlx prepare --workspace {{ARGS}} -- --tests

test *ARGS:
	cargo test --workspace {{ARGS}} -- --nocapture

run *ARGS:
	cargo run {{ARGS}}

run-with-console *ARGS:
	RUSTFLAGS="--cfg tokio_unstable" cargo +nightly run --features console {{ARGS}}

database:
	docker compose up --detach --wait cs2kz-database

clean-database:
	docker compose down --timeout 1 cs2kz-database
	sudo rm -rf ./database/volumes/cs2kz

database-connection:
	#!/usr/bin/env bash
	if command -v mycli &>/dev/null; then
	  export cmd=mycli
	elif command -v mariadb &>/dev/null; then
	  export cmd=mariadb
	else
	  export cmd=mysql
	fi

	export cmd="$cmd \
	  -u schnose \
	  -pcsgo-kz-is-dead-boys \
	  -h 127.0.0.1 \
	  -P "$DATABASE_PORT" \
	  -D cs2kz
	"

	eval "$cmd"

migrations *ARGS:
	sqlx migrate run --source {{justfile_directory()}}/database/migrations {{ARGS}}

precommit:
	just clippy
	just fmt
	just doc
	just sqlx-cache
