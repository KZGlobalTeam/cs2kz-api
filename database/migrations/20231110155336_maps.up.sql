/**
 * A KZ map corresponds to a CS2 map.
 *
 * Each map can contain multiple KZ courses.
 * Each map is considered to be created / owned by a single player.
 * Each map has to be uploaded to the Steam workshop, preferably by the player who made it.
 *
 * Courses individually can have multiple players associated with them who helped with mapping those
 * courses.
 */
CREATE TABLE IF NOT EXISTS Maps (
	`id` INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
	`name` VARCHAR(32) NOT NULL,
	`workshop_id` INT4 UNSIGNED NOT NULL,
	`filesize` INT8 UNSIGNED NOT NULL,
	`created_by` INT4 UNSIGNED NOT NULL,
	`created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY (`id`),
	FOREIGN KEY (`created_by`) REFERENCES Players(`steam_id`)
);
