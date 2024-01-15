/**
 * This table holds users that some elevated permissions.
 * This includes admins responsible for bans, server approval, map approval, etc.
 *
 * The `role_flags` column is a bitfield.
 * For more information about the bitfield values, check the Rust code.
 */
CREATE TABLE IF NOT EXISTS Admins (
	`steam_id` INT4 UNSIGNED NOT NULL,
	`role_flags` INT4 UNSIGNED NOT NULL,
	PRIMARY KEY (`steam_id`, `role_flags`),
	FOREIGN KEY (`steam_id`) REFERENCES Players (`steam_id`),
  UNIQUE (`steam_id`)
);

/**
 * This table holds session tokens for the various websites on `*.cs2.kz` subdomains.
 *
 * If `subdomain` is NULL, the request is considered to have originated from the main
 * `cs2.kz` site.
 */
CREATE TABLE IF NOT EXISTS WebSessions (
  `id` INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
  `subdomain` VARCHAR(16),
  `token` INT8 UNSIGNED NOT NULL,
  `steam_id` INT4 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `expires_on` TIMESTAMP NOT NULL,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`steam_id`) REFERENCES Players (`steam_id`),
  UNIQUE (`token`)
);
