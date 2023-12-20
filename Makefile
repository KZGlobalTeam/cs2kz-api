include .env.example
-include .env
export

balls:
	@echo $${DATABASE_URL}

default:
	@make db
	@echo "Waiting for the database to spin up..."
	@sleep 10s
	@make migrations
	@make api

db:
	@echo "Starting database container..."
	@docker compose up -d --wait cs2kz-database

db-clean:
	rm -rf ./database/volumes/cs2kz

db-connect:
	@echo "Connecting to database..."
	@mariadb \
		-u kz \
		-pcsgo-kz-is-dead-boys \
		-h 127.0.0.1 \
		-P $${KZ_API_DATABASE_PORT:-8070} \
		-D cs2kz

migrations:
	@echo "Running migrations..."
	@sqlx migrate run \
		--source ./database/migrations/ \
		--database-url $${DATABASE_URL}

api:
	@echo "Building API container..."
	@docker compose build cs2kz-api
	@echo "Running API..."
	@docker compose up -d --wait cs2kz-api
	@docker compose logs --follow cs2kz-api

api-spec:
	@echo "Generating OpenAPI docs..."
	cargo run --package cs2kz-api-spec-generator -- --output api-spec.json

sqlx-cache:
	cargo sqlx prepare \
		--workspace \
		--database-url $${DATABASE_URL}

dev:
	cargo run -p cs2kz-api

check:
	cargo clippy --all-features --workspace -- -D warnings

fmt:
	cargo +nightly fmt --all

fmt-check:
	cargo +nightly fmt --all --check

docs:
	@echo "Documenting all crates..."
	cargo doc --all-features --workspace --document-private-items --no-deps
	@echo "Checking if the OpenAPI docs are up to date..."
	cargo run --package cs2kz-api-spec-generator -- --check api-spec.json

test:
	@make db
	DATABASE_URL=$${TEST_DATABASE_URL} cargo test --package cs2kz-api $(ARGS) -- --nocapture
