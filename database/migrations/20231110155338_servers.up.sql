/**
 * These are officially approved servers that are allowed to submit records and ban players for
 * cheating.
 *
 * The `ip_address` and `port` are expected to be kept up to date by the server owner.
 * The `api_key` is randomly generated and is used for authentication. The server owner should be
 * able to reset this key. It may be NULL if it hasn't been generated yet or has been revoked.
 */
CREATE TABLE IF NOT EXISTS Servers (
  `id` INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
  `name` VARCHAR(255) NOT NULL,
  `ip_address` INET4 NOT NULL,
  `port` INT2 UNSIGNED NOT NULL,
  `owned_by` INT4 UNSIGNED NOT NULL,
  `api_key` INT4 UNSIGNED,
  `approved_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`owned_by`) REFERENCES Players (`steam_id`),
  UNIQUE (`name`),
  UNIQUE (`api_key`)
);
