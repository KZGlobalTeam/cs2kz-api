include .env.example
-include .env
export

DATABASE_PORT ?= "8070"

api:
	@echo "Building API container..."
	@docker compose build cs2kz-api
	@echo "Running API..."
	@docker compose up -d --wait cs2kz-api
	@docker compose logs --follow cs2kz-api

db:
	@echo "Starting database container..."
	@docker compose up -d --wait cs2kz-database

db-clean:
	docker compose down -t 1 cs2kz-database
	rm -rf ./database/volumes/cs2kz

db-connect:
	@echo "Connecting to database..."
	@$(if $(shell command -v mycli 2>/dev/null), mycli, mariadb) \
		-u kz \
		-pcsgo-kz-is-dead-boys \
		-h 127.0.0.1 \
		-P $(DATABASE_PORT) \
		-D cs2kz

db-connect-root:
	@echo "Connecting to database as root..."
	@$(if $(shell command -v mycli 2>/dev/null), mycli, mariadb) \
		-u root \
		-pcsgo-kz-is-dead-boys \
		-h 127.0.0.1 \
		-P $(DATABASE_PORT) \
		-D cs2kz

api-spec:
	@echo "Generating OpenAPI docs..."
	cargo run --package spec-generator -- --output api-spec.json

sqlx-cache:
	cargo sqlx prepare \
		--workspace \
		--database-url $(DATABASE_URL)

dev:
	cargo run -p cs2kz-api

dev-debug:
	RUSTFLAGS="--cfg tokio_unstable" cargo run -p cs2kz-api -F console

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
	cargo run --package spec-generator -- --check api-spec.json

test:
	@make db
	DATABASE_URL=$(TEST_DATABASE_URL) cargo test --package cs2kz-api $(ARGS) -- --nocapture
