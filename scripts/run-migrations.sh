#!/bin/sh

for file in $(ls ./database/migrations | grep ".up"); do
	eval $(grep -v '^#' .env.example | xargs); mariadb \
		-u root \
		-pcsgo-kz-is-dead-boys \
		-h 127.0.0.1 \
		-P "$DATABASE_PORT" \
		-D cs2kz < "./database/migrations/$file"
done
