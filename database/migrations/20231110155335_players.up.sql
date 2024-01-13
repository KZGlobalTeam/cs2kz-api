/**
 * Players are tied to their Steam account.
 *
 * They are uniquely identified by their SteamID and we store their last known IP address for the
 * sake of IP bans.
 *
 * The `steam_id` column stores SteamIDs represented in their 32-bit format.
 * See https://developer.valvesoftware.com/wiki/SteamID
 */
CREATE TABLE IF NOT EXISTS Players (
  `steam_id` INT4 UNSIGNED NOT NULL,
  `name` VARCHAR(32) NOT NULL,
  `last_known_ip_address` INET4 NOT NULL,
  `is_banned` BOOLEAN NOT NULL DEFAULT FALSE,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`steam_id`)
);
