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
	sqlx migrate run --source ./database/migrations/

api:
	docker compose build cs2kz-api

run:
	docker compose up

dev:
	cargo run

format:
	cargo +nightly fmt --all

lint:
	cargo clippy --all-features --workspace -- -D warnings
