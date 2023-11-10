/**
 * A course is a single unit of gameplay on a KZ map.
 */
CREATE TABLE IF NOT EXISTS Courses (
	`id` INT4 UNSIGNED NOT NULL AUTO_INCREMENT,
	`map_id` INT2 UNSIGNED NOT NULL,
	`map_stage` INT1 UNSIGNED NOT NULL,
	PRIMARY KEY (`id`),
	FOREIGN KEY (`map_id`) REFERENCES Maps (`id`)
);

/**
 * Any course can have one or more mappers, and any player can be the mapper of any amount of
 * courses.
 */
CREATE TABLE IF NOT EXISTS CourseMappers (
	`course_id` INT4 UNSIGNED NOT NULL,
	`player_id` INT4 UNSIGNED NOT NULL,
	PRIMARY KEY (`course_id`, `player_id`),
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
	`has_teleports` BOOLEAN NOT NULL,
	PRIMARY KEY (`id`),
	FOREIGN KEY (`course_id`) REFERENCES Courses (`id`),
	FOREIGN KEY (`mode_id`) REFERENCES Modes (`id`)
);
