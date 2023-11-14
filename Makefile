default:
	make db
	make migrations
	make api
	@echo ""
	@echo "Finished setting up. You can run the API container via \`make run\`."
	@echo ""

clean:
	docker compose down --rmi all

db-clean:
	sudo rm -rf ./database/volumes/cs2kz-database/

db:
	docker compose up -d --wait cs2kz-database

migrations:
	sqlx migrate run \
		--source ./database/migrations/ \
		--database-url mysql://kz:csgo-kz-is-dead-boys@127.0.0.1:8070/cs2kz-api

sqlx-data:
	cargo sqlx prepare --workspace

api:
	docker compose build cs2kz-api

run:
	docker compose up

dev:
	DATABASE_URL=mysql://kz:csgo-kz-is-dead-boys@127.0.0.1:8070/cs2kz-api cargo run -p cs2kz-api

format:
	cargo +nightly fmt --all

lint:
	cargo clippy --all-features --workspace -- -D warnings

spec:
	cargo run -p cs2kz-api-spec-generator -- json

spec-check:
	cargo run -p cs2kz-api-spec-generator -- check
