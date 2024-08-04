set dotenv-load := true

rustfmt := if env('IN_DEV_SHELL', '0') == '1' { 'cargo fmt' } else { 'cargo +nightly fmt' }

# List all available recipes
help:
	@just --list

# Various integrity checks
check:
	# Running clippy...
	cargo clippy --workspace --all-features --tests --no-deps -- -Dwarnings

	# Running rustfmt...
	{{rustfmt}} --all --check

	# Running rustdoc...
	RUSTDOCFLAGS="-Dwarnings" cargo doc --workspace --all-features --document-private-items

	# Running sqlx...
	cargo sqlx prepare --workspace --check -- --tests

# Format the code
format:
	# Running rustfmt...
	{{rustfmt}} --all

# Run with tokio-console support
debug *ARGS:
	RUSTFLAGS="--cfg tokio_unstable" cargo run -Fconsole serve {{ARGS}}

# Spin up the database container
create-database:
	docker compose up --detach --wait cs2kz-database

# Remove the database container and clean volumes
clean-database:
	docker compose down --timeout=3 cs2kz-database
	sudo rm -rfv {{justfile_directory()}}/database/volumes/cs2kz

# Run database migrations
run-migrations:
	cargo sqlx migrate run --source {{justfile_directory()}}/database/migrations

# Build sqlx's query cache
prepare-query-cache:
	cargo sqlx prepare --workspace -- --tests
