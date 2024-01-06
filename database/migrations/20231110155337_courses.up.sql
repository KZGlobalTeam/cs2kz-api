/**
 * A course is a single unit of gameplay on a KZ map.
 */
CREATE TABLE IF NOT EXISTS Courses (
  `id` INT4 UNSIGNED NOT NULL AUTO_INCREMENT,
  `map_id` INT2 UNSIGNED NOT NULL,
  `map_stage` INT1 UNSIGNED NOT NULL,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`map_id`) REFERENCES Maps (`id`),
  UNIQUE (`map_id`, `map_stage`),
  CONSTRAINT `valid_stage` CHECK(`map_stage` > 0)
);

/**
 * Any course has one or more mappers, and any player can be the mapper of any amount of courses.
 */
CREATE TABLE IF NOT EXISTS CourseMappers (
  `course_id` INT4 UNSIGNED NOT NULL,
  `player_id` INT4 UNSIGNED NOT NULL,
  PRIMARY KEY (`course_id`, `player_id`),
  FOREIGN KEY (`course_id`) REFERENCES Courses (`id`),
  FOREIGN KEY (`player_id`) REFERENCES Players (`steam_id`)
);

/**
 * Courses are considered "ranked" for pairs of a mode and whether teleports can be used.
 * One such pair is called a "filter".
 *
 * Records may only be submitted for courses that have a filter.
 */
CREATE TABLE IF NOT EXISTS CourseFilters (
  `id` INT4 UNSIGNED NOT NULL AUTO_INCREMENT,
  `course_id` INT4 UNSIGNED NOT NULL,
  `mode_id` INT1 UNSIGNED NOT NULL,
  `teleports` BOOLEAN NOT NULL,
  `tier` INT1 UNSIGNED NOT NULL,
  `ranked_status` INT1 NOT NULL,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`course_id`) REFERENCES Courses (`id`),
  FOREIGN KEY (`mode_id`) REFERENCES Modes (`id`),
  CONSTRAINT `valid_tier` CHECK(`tier` BETWEEN 1 AND 10),
  CONSTRAINT `valid_ranked_status` CHECK(`ranked_status` BETWEEN -1 AND 1)
);
