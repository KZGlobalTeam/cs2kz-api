/**
 * Record or "run" submissions.
 *
 * If a player completes a map on an approved KZ server, that run will be submitted to the API and
 * inserted into the database. Records can then be compared against each other to form leaderboards
 * etc.
 *
 * Each submitted record also has a replay attached to it which can be used to determine if a run is
 * cheated or not. Records are bucketed into `Records`, `SuspiciousRecords` and `CheatedRecords`
 * depending on their legitimacy status.
 */
CREATE TABLE IF NOT EXISTS Records (
  `id` INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
  `player_id` INT4 UNSIGNED NOT NULL,
  `filter_id` INT4 UNSIGNED NOT NULL,
  `style_id` INT1 UNSIGNED NOT NULL,
  `teleports` INT2 UNSIGNED NOT NULL,
  `time` FLOAT8 NOT NULL,
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
  `plugin_version_id` INT2 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`player_id`) REFERENCES Players (`steam_id`),
  FOREIGN KEY (`filter_id`) REFERENCES CourseFilters (`id`),
  FOREIGN KEY (`style_id`) REFERENCES Styles (`id`),
  FOREIGN KEY (`server_id`) REFERENCES Servers (`id`),
  FOREIGN KEY (`plugin_version_id`) REFERENCES PluginVersions (`id`)
);

CREATE TABLE IF NOT EXISTS RecordSplits (
  `id` INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
  `record_id` INT8 UNSIGNED NOT NULL,
  `split_id` INT8 UNSIGNED NOT NULL,
  `time` FLOAT8 NOT NULL,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`record_id`) REFERENCES Records (`id`),
  FOREIGN KEY (`split_id`) REFERENCES CourseSplits (`id`)
);

CREATE TABLE IF NOT EXISTS SuspiciousRecords AS
SELECT
  *
FROM
  Records;

CREATE TABLE IF NOT EXISTS CheatedRecords AS
SELECT
  *
FROM
  Records;
