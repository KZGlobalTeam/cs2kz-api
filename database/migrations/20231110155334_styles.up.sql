/**
 * KZ allows for performing runs or jumpstats using different "styles" of play.
 *
 * The rows in this table are represented as an enum in Rust.
 * See `cs2kz::Style` for more information.
 */
CREATE TABLE IF NOT EXISTS Styles (
	`id` INT1 UNSIGNED NOT NULL AUTO_INCREMENT,
	`name` VARCHAR(16) NOT NULL,
	`created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY (`id`)
);

/**
 * The default style.
 */
INSERT INTO
	Styles (`name`)
VALUES
	("normal");

/**
 * Directions are inverted.
 *
 * W -> S
 * A -> D
 * S -> W
 * D -> A
 */
INSERT INTO
	Styles (`name`)
VALUES
	("backwards");

/**
 * Directions are rotated by 1 position.
 *
 * W -> D (or A)
 * A -> W (or S)
 * S -> A (or D)
 * D -> S (or W)
 */
INSERT INTO
	Styles (`name`)
VALUES
	("sideways");

/**
 * Only the W key is used for strafing.
 */
INSERT INTO
	Styles (`name`)
VALUES
	("w_only");
