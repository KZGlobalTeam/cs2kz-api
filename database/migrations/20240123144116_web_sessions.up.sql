/**
 * This table holds session tokens for the various websites on `*.cs2.kz` subdomains.
 *
 * If `subdomain` is NULL, the request is considered to have originated from the main
 * `cs2.kz` site.
 */
CREATE TABLE IF NOT EXISTS WebSessions (
  `id` INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
  `token` INT8 UNSIGNED NOT NULL,
  `service_id` INT8 UNSIGNED NOT NULL,
  `steam_id` INT4 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `expires_on` TIMESTAMP NOT NULL DEFAULT DATE_ADD(NOW(), INTERVAL 7 DAY),
  PRIMARY KEY (`id`),
  FOREIGN KEY (`service_id`) REFERENCES Services (`id`),
  FOREIGN KEY (`steam_id`) REFERENCES Players (`steam_id`),
  UNIQUE (`token`)
);
