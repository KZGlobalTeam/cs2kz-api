/**
 * This table holds session tokens for the various websites on `*.cs2.kz` subdomains.
 */
CREATE TABLE IF NOT EXISTS WebSessions (
  `id` INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
  `token` INT8 UNSIGNED NOT NULL,
  `steam_id` INT4 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `expires_on` TIMESTAMP NOT NULL,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`steam_id`) REFERENCES Players (`steam_id`),
  UNIQUE (`token`)
);
