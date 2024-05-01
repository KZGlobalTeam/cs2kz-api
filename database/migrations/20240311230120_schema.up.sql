CREATE TABLE IF NOT EXISTS `PluginVersions` (
  `id` INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
  `semver` VARCHAR(14) NOT NULL,
  `git_revision` VARCHAR(255) NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE (`semver`),
  UNIQUE (`git_revision`)
);

CREATE TABLE IF NOT EXISTS `Credentials` (
  `id` INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
  `name` VARCHAR(255) NOT NULL,
  `token` UUID NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `expires_on` TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE (`token`)
);

CREATE TABLE IF NOT EXISTS `Modes` (
  `id` INT1 UNSIGNED NOT NULL AUTO_INCREMENT,
  `name` VARCHAR(16) NOT NULL,
  PRIMARY KEY (`id`)
);

CREATE TABLE IF NOT EXISTS `Styles` (
  `id` INT1 UNSIGNED NOT NULL AUTO_INCREMENT,
  `name` VARCHAR(16) NOT NULL,
  PRIMARY KEY (`id`)
);

CREATE TABLE IF NOT EXISTS `JumpTypes` (
  `id` INT1 UNSIGNED NOT NULL AUTO_INCREMENT,
  `name` VARCHAR(16) NOT NULL,
  PRIMARY KEY (`id`)
);

CREATE TABLE IF NOT EXISTS `Players` (
  `id` INT8 UNSIGNED NOT NULL,
  `name` VARCHAR(32) NOT NULL,
  `ip_address` INET4 NOT NULL,
  `role_flags` INT8 UNSIGNED NOT NULL DEFAULT 0,
  `preferences` JSON NOT NULL DEFAULT "{}",
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `last_seen_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`)
);

CREATE TABLE IF NOT EXISTS `Maps` (
  `id` INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
  `name` VARCHAR(32) NOT NULL,
  `description` TEXT,
  `global_status` INT1 NOT NULL DEFAULT -1,
  `workshop_id` INT4 UNSIGNED NOT NULL,
  `checksum` INT4 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  CONSTRAINT `valid_global_status` CHECK(`global_status` BETWEEN -1 AND 1)
);

CREATE TABLE IF NOT EXISTS `Mappers` (
  `map_id` INT2 UNSIGNED NOT NULL,
  `player_id` INT8 UNSIGNED NOT NULL,
  PRIMARY KEY (`map_id`, `player_id`),
  FOREIGN KEY (`map_id`) REFERENCES `Maps` (`id`) ON DELETE CASCADE,
  FOREIGN KEY (`player_id`) REFERENCES `Players` (`id`)
);

CREATE TABLE IF NOT EXISTS `Courses` (
  `id` INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
  `name` VARCHAR(16) NOT NULL,
  `description` TEXT,
  `map_id` INT2 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`map_id`) REFERENCES `Maps` (`id`) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS `CourseMappers` (
  `course_id` INT2 UNSIGNED NOT NULL,
  `player_id` INT8 UNSIGNED NOT NULL,
  PRIMARY KEY (`course_id`, `player_id`),
  FOREIGN KEY (`course_id`) REFERENCES `Courses` (`id`) ON DELETE CASCADE,
  FOREIGN KEY (`player_id`) REFERENCES `Players` (`id`)
);

CREATE TABLE IF NOT EXISTS `CourseFilters` (
  `id` INT4 UNSIGNED NOT NULL AUTO_INCREMENT,
  `course_id` INT2 UNSIGNED NOT NULL,
  `mode_id` INT1 UNSIGNED NOT NULL,
  `teleports` BOOLEAN NOT NULL,
  `tier` INT1 UNSIGNED NOT NULL,
  `ranked_status` INT1 NOT NULL DEFAULT -1,
  `notes` TEXT,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`course_id`) REFERENCES `Courses` (`id`) ON DELETE CASCADE,
  FOREIGN KEY (`mode_id`) REFERENCES `Modes` (`id`),
  CONSTRAINT `valid_tier` CHECK(`tier` BETWEEN 1 AND 10),
  CONSTRAINT `valid_ranked_status` CHECK(
    (`ranked_status` BETWEEN -1 AND 1)
    AND (
      (
        `tier` > 8
        AND `ranked_status` < 1
      )
      OR TRUE
    )
  ),
  UNIQUE (`course_id`, `mode_id`, `teleports`)
);

CREATE TABLE IF NOT EXISTS `Servers` (
  `id` INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
  `name` VARCHAR(255) NOT NULL,
  `ip_address` INET4 NOT NULL,
  `port` INT2 UNSIGNED NOT NULL,
  `owner_id` INT8 UNSIGNED NOT NULL,
  `refresh_key` UUID,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `last_seen_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`owner_id`) REFERENCES `Players` (`id`),
  UNIQUE (`name`),
  UNIQUE (`ip_address`, `port`),
  UNIQUE (`refresh_key`)
);

CREATE TABLE IF NOT EXISTS `Jumpstats` (
  `id` INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
  `type` INT1 UNSIGNED NOT NULL,
  `mode_id` INT1 UNSIGNED NOT NULL,
  `style_id` INT1 UNSIGNED NOT NULL,
  `strafes` INT1 UNSIGNED NOT NULL,
  `distance` FLOAT4 NOT NULL,
  `sync` FLOAT4 NOT NULL,
  `pre` FLOAT4 NOT NULL,
  `max` FLOAT4 NOT NULL,
  `overlap` FLOAT4 NOT NULL,
  `bad_angles` FLOAT4 NOT NULL,
  `dead_air` FLOAT4 NOT NULL,
  `height` FLOAT4 NOT NULL,
  `airpath` FLOAT4 NOT NULL,
  `deviation` FLOAT4 NOT NULL,
  `average_width` FLOAT4 NOT NULL,
  `airtime` FLOAT4 NOT NULL,
  `player_id` INT8 UNSIGNED NOT NULL,
  `server_id` INT2 UNSIGNED NOT NULL,
  `legitimacy` INT1 UNSIGNED NOT NULL,
  `plugin_version_id` INT2 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`mode_id`) REFERENCES `Modes` (`id`),
  FOREIGN KEY (`style_id`) REFERENCES `Styles` (`id`),
  FOREIGN KEY (`player_id`) REFERENCES `Players` (`id`),
  FOREIGN KEY (`server_id`) REFERENCES `Servers` (`id`),
  FOREIGN KEY (`plugin_version_id`) REFERENCES `PluginVersions` (`id`),
  CONSTRAINT `valid_legitimacy` CHECK(`legitimacy` BETWEEN 0 AND 2)
);

CREATE TABLE IF NOT EXISTS `Records` (
  `id` INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
  `filter_id` INT4 UNSIGNED NOT NULL,
  `style_flags` INT4 UNSIGNED NOT NULL,
  `teleports` INT2 UNSIGNED NOT NULL,
  `time` FLOAT8 NOT NULL,
  `player_id` INT8 UNSIGNED NOT NULL,
  `server_id` INT2 UNSIGNED NOT NULL,
  `perfs` INT2 UNSIGNED NOT NULL,
  `bhops_tick0` INT2 UNSIGNED NOT NULL,
  `bhops_tick1` INT2 UNSIGNED NOT NULL,
  `bhops_tick2` INT2 UNSIGNED NOT NULL,
  `bhops_tick3` INT2 UNSIGNED NOT NULL,
  `bhops_tick4` INT2 UNSIGNED NOT NULL,
  `bhops_tick5` INT2 UNSIGNED NOT NULL,
  `bhops_tick6` INT2 UNSIGNED NOT NULL,
  `bhops_tick7` INT2 UNSIGNED NOT NULL,
  `bhops_tick8` INT2 UNSIGNED NOT NULL,
  `legitimacy` INT1 UNSIGNED NOT NULL,
  `plugin_version_id` INT2 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`filter_id`) REFERENCES `CourseFilters` (`id`),
  FOREIGN KEY (`player_id`) REFERENCES `Players` (`id`),
  FOREIGN KEY (`server_id`) REFERENCES `Servers` (`id`),
  FOREIGN KEY (`plugin_version_id`) REFERENCES `PluginVersions` (`id`),
  CONSTRAINT `valid_legitimacy` CHECK(`legitimacy` BETWEEN 0 AND 2)
);

CREATE TABLE IF NOT EXISTS `Bans` (
  `id` INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
  `player_id` INT8 UNSIGNED NOT NULL,
  `player_ip` INET4 NOT NULL,
  `server_id` INT2 UNSIGNED,
  `reason` VARCHAR(32) NOT NULL,
  `admin_id` INT8 UNSIGNED,
  `plugin_version_id` INT2 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `expires_on` TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`player_id`) REFERENCES `Players` (`id`),
  FOREIGN KEY (`server_id`) REFERENCES `Servers` (`id`),
  FOREIGN KEY (`admin_id`) REFERENCES `Players` (`id`),
  FOREIGN KEY (`plugin_version_id`) REFERENCES `PluginVersions` (`id`)
);

CREATE TABLE IF NOT EXISTS `Unbans` (
  `id` INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
  `ban_id` INT8 UNSIGNED NOT NULL,
  `reason` TEXT NOT NULL,
  `admin_id` INT8 UNSIGNED,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`ban_id`) REFERENCES `Bans` (`id`),
  FOREIGN KEY (`admin_id`) REFERENCES `Players` (`id`),
  UNIQUE (`ban_id`)
);

CREATE TABLE IF NOT EXISTS `GameSessions` (
  `id` INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
  `player_id` INT8 UNSIGNED NOT NULL,
  `server_id` INT2 UNSIGNED NOT NULL,
  `time_active` INT2 NOT NULL,
  `time_spectating` INT2 NOT NULL,
  `time_afk` INT2 NOT NULL,
  `perfs` INT2 UNSIGNED NOT NULL,
  `bhops_tick0` INT2 UNSIGNED NOT NULL,
  `bhops_tick1` INT2 UNSIGNED NOT NULL,
  `bhops_tick2` INT2 UNSIGNED NOT NULL,
  `bhops_tick3` INT2 UNSIGNED NOT NULL,
  `bhops_tick4` INT2 UNSIGNED NOT NULL,
  `bhops_tick5` INT2 UNSIGNED NOT NULL,
  `bhops_tick6` INT2 UNSIGNED NOT NULL,
  `bhops_tick7` INT2 UNSIGNED NOT NULL,
  `bhops_tick8` INT2 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`player_id`) REFERENCES `Players` (`id`),
  FOREIGN KEY (`server_id`) REFERENCES `Servers` (`id`)
);

CREATE TABLE IF NOT EXISTS `CourseSessions` (
  `id` INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
  `player_id` INT8 UNSIGNED NOT NULL,
  `course_id` INT2 UNSIGNED NOT NULL,
  `mode_id` INT1 UNSIGNED NOT NULL,
  `server_id` INT2 UNSIGNED NOT NULL,
  `playtime` INT2 NOT NULL,
  `started_runs` INT2 UNSIGNED NOT NULL,
  `finished_runs` INT2 UNSIGNED NOT NULL,
  `perfs` INT2 UNSIGNED NOT NULL,
  `bhops_tick0` INT2 UNSIGNED NOT NULL,
  `bhops_tick1` INT2 UNSIGNED NOT NULL,
  `bhops_tick2` INT2 UNSIGNED NOT NULL,
  `bhops_tick3` INT2 UNSIGNED NOT NULL,
  `bhops_tick4` INT2 UNSIGNED NOT NULL,
  `bhops_tick5` INT2 UNSIGNED NOT NULL,
  `bhops_tick6` INT2 UNSIGNED NOT NULL,
  `bhops_tick7` INT2 UNSIGNED NOT NULL,
  `bhops_tick8` INT2 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`player_id`) REFERENCES `Players` (`id`),
  FOREIGN KEY (`course_id`) REFERENCES `Courses` (`id`) ON DELETE CASCADE,
  FOREIGN KEY (`mode_id`) REFERENCES `Modes` (`id`),
  FOREIGN KEY (`server_id`) REFERENCES `Servers` (`id`)
);

CREATE TABLE IF NOT EXISTS `LoginSessions` (
  `id` UUID NOT NULL,
  `player_id` INT8 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `expires_on` TIMESTAMP NOT NULL,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`player_id`) REFERENCES `Players` (`id`)
);
