/**
 * Whenever a player is found cheating or breaking other rules they will be excluded from playing on
 * approved servers (unless the server owner allows them to) and submitting records.
 *
 * These bans are kept track of even if they are reverted again.
 * Each ban has a replay associated with it of the last few minutes of gameplay before the player
 * got banned. This makes judging whether a player actually broke the rules or not significantly
 * easier.
 *
 * A player may be banned while playing on a server, in which case the `player_ip` and `server_id`
 * columns should be populated. An admin may also ban a player externally at any point in time, in
 * which case the player's last known IP address is used for `player_ip` and `server_id` is left as
 * NULL.
 */
CREATE TABLE IF NOT EXISTS Bans (
  `id` INT4 UNSIGNED NOT NULL AUTO_INCREMENT,
  `player_id` INT4 UNSIGNED NOT NULL,
  `player_ip` INET4 NOT NULL,
  `reason` VARCHAR(255) NOT NULL,
  `server_id` INT2 UNSIGNED,
  `plugin_version_id` INT2 UNSIGNED NOT NULL,
  `banned_by` INT4 UNSIGNED,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `expires_on` TIMESTAMP NOT NULL DEFAULT DATE_ADD(NOW(), INTERVAL 10 YEAR),
  PRIMARY KEY (`id`),
  FOREIGN KEY (`player_id`) REFERENCES Players (`steam_id`),
  FOREIGN KEY (`server_id`) REFERENCES Servers (`id`),
  FOREIGN KEY (`plugin_version_id`) REFERENCES PluginVersions (`id`),
  FOREIGN KEY (`banned_by`) REFERENCES Players (`steam_id`)
);

/**
 * A ban being lifted is also something being kept track of.
 * This may happen automatically when a ban expires, or manually because a ban was found to be
 * invalid.
 */
CREATE TABLE IF NOT EXISTS Unbans (
  `id` INT4 UNSIGNED NOT NULL AUTO_INCREMENT,
  `ban_id` INT4 UNSIGNED NOT NULL,
  `reason` VARCHAR(1024) NOT NULL DEFAULT "expired",
  `unbanned_by` INT4 UNSIGNED,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`ban_id`) REFERENCES Bans (`id`) ON DELETE CASCADE,
  FOREIGN KEY (`unbanned_by`) REFERENCES Players (`steam_id`),
  UNIQUE (`ban_id`)
);
