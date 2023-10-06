#!/bin/sh

mariadb -u kz \
	-pcsgo-kz-is-dead-boys \
	-h 127.0.0.1 \
	-P 8070 \
	-D cs2kz-api
