/**
 * A KZ map corresponds to a CS2 map.
 *
 * Each map can contain multiple KZ courses.
 * Each map has to be uploaded to the Steam workshop, preferably by the player who made it.
 * Each map can have multiple "mappers" associated with it who are credited as the creators of the
 * map.
 */
CREATE TABLE IF NOT EXISTS Maps (
  `id` INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
  `name` VARCHAR(32) NOT NULL,
  `workshop_id` INT4 UNSIGNED NOT NULL,
  `checksum` INT4 UNSIGNED NOT NULL,
  `is_global` BOOLEAN NOT NULL DEFAULT TRUE,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`)
);

/**
 * Any map has one or more mappers.
 */
CREATE TABLE IF NOT EXISTS Mappers (
  `map_id` INT2 UNSIGNED NOT NULL,
  `player_id` INT4 UNSIGNED NOT NULL,
  PRIMARY KEY (`map_id`, `player_id`),
  FOREIGN KEY (`map_id`) REFERENCES Maps (`id`),
  FOREIGN KEY (`player_id`) REFERENCES Players (`steam_id`)
);
