/**
 * KZ allows for playing in multiple different modes which have different movement settings.
 *
 * The rows in this table are represented as an enum in Rust.
 * See `cs2kz::Mode` for more information.
 */
CREATE TABLE IF NOT EXISTS Modes (
	`id` INT1 UNSIGNED NOT NULL AUTO_INCREMENT,
	`name` VARCHAR(16) NOT NULL,
	`created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY (`id`),
	CONSTRAINT `valid_name` CHECK(`name` LIKE "kz_%")
);

/**
 * Default CS2 gameplay.
 *
 * The only changes made to this mode are QoL; nothing that drastically changes the movement itself.
 */
INSERT INTO
	Modes (`name`)
VALUES
	("kz_vanilla");

/**
 * Heavily modified movement compared to vanilla gameplay.
 */
INSERT INTO
	Modes (`name`)
VALUES
	("kz_modded");
