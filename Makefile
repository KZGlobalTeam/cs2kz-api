build:
	# Setting up database...
	@make db

	# Running database migrations...
	@make migrations

	# Setting up API...
	@make api

	# Finished setup. Use \`make run\` to run the API.

clean:
	# Stopping containers...
	docker-compose down

	# Removing containers...
	docker container prune

	# Removing images...
	docker image prune

wipe:
	# Cleaning up...
	@make clean

	# !!! Deleting database !!!
	sudo rm -rf ./database/volumes

db:
	docker-compose up -d cs2kz-database

migrations:
	sqlx migrate run --source database/migrations/

api:
	docker-compose build cs2kz-api

run:
	docker-compose up

dev:
	cargo run

lint:
	cargo clippy --all-features --workspace -- -D warnings

format:
	cargo +nightly fmt --all
