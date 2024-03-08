/**
 * A session begins when a player joins a server and ends when they leave.
 *
 * This is used to keep track of total time spent playing KZ among other stats.
 */
CREATE TABLE IF NOT EXISTS Sessions (
  `id` INT4 UNSIGNED NOT NULL AUTO_INCREMENT,
  `player_id` INT4 UNSIGNED NOT NULL,
  `server_id` INT2 UNSIGNED NOT NULL,
  `time_active` INT2 UNSIGNED NOT NULL,
  `time_spectating` INT2 UNSIGNED NOT NULL,
  `time_afk` INT2 UNSIGNED NOT NULL,
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
  FOREIGN KEY (`player_id`) REFERENCES Players (`steam_id`),
  FOREIGN KEY (`server_id`) REFERENCES Servers (`id`)
);

/**
 * A course session begins when a player starts running a specific course and ends when they start
 * running a different course.
 */
CREATE TABLE IF NOT EXISTS CourseSessions (
  `id` INT4 UNSIGNED NOT NULL AUTO_INCREMENT,
  `player_id` INT4 UNSIGNED NOT NULL,
  `course_id` INT4 UNSIGNED NOT NULL,
  `mode_id` INT1 UNSIGNED NOT NULL,
  `server_id` INT2 UNSIGNED NOT NULL,
  `playtime` INT2 UNSIGNED NOT NULL,
  `total_runs` INT2 UNSIGNED NOT NULL,
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
  FOREIGN KEY (`player_id`) REFERENCES Players (`steam_id`),
  FOREIGN KEY (`course_id`) REFERENCES Courses (`id`) ON DELETE CASCADE,
  FOREIGN KEY (`mode_id`) REFERENCES Modes (`id`),
  FOREIGN KEY (`server_id`) REFERENCES Servers (`id`)
);
