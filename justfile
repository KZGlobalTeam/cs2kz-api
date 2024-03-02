set dotenv-load := true

# list of all available commands
help:
	@just --list

# ensure database is running
create-database:
	docker compose up --detach --wait cs2kz-database

# shut down the database and remove persistent data
clean-database:
	docker compose down --timeout 1 cs2kz-database
	rm -rf {{justfile_directory()}}/database/volumes/cs2kz

# connect to the database from the cli
connect-to-database:
	@mariadb \
		-u kz \
		-pcsgo-kz-is-dead-boys \
		-h 127.0.0.1 \
		-P "$DATABASE_PORT" \
		-D cs2kz

# run database migrations
run-migrations:
	@if ! command -v sqlx &> /dev/null; then \
		echo "You do not have sqlx-cli installed."; \
		exit 1; \
	fi

	sqlx migrate run \
		--source {{justfile_directory()}}/database/migrations/ \
		--database-url "$DATABASE_URL"

# run static analysis
check:
	cargo clippy --all-features --tests

# run tests
test *TEST_ARGS:
	just create-database
	cargo test {{TEST_ARGS}} -- --nocapture

# format the codebase
format:
	cargo +nightly fmt --all

# document the codebase
document *DOC_ARGS:
	cargo doc --all-features --document-private-items {{DOC_ARGS}}
	cargo run --bin spec-generator -- --check {{justfile_directory()}}/api-spec.json

# generate `api-spec.json`
generate-open-api-spec:
	cargo run --bin spec-generator -- --output {{justfile_directory()}}/api-spec.json

# create query cache for sqlx
sqlx-cache:
	@just create-database
	cargo sqlx prepare -- --tests

# run the API locally
run:
	@just create-database
	cargo run --bin api

# run the API locally with tokio-console
run-with-console:
	@just create-database
	RUSTFLAGS="--cfg tokio_unstable" cargo run --bin api --features console

# run the API with docker
deploy:
	just create-database
	docker compose build --build-arg DEPOT_DOWNLOADER_URL=https://github.com/SteamRE/DepotDownloader/releases/download/DepotDownloader_2.5.0/DepotDownloader-linux-arm64.zip cs2kz-api
	docker compose up --detach --wait --force-recreate cs2kz-api
	docker compose logs --follow cs2kz-api
