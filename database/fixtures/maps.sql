/* kz_checkmate */
INSERT
	IGNORE INTO Maps (`name`, `workshop_id`, `filesize`)
VALUES
	("kz_checkmate", 3070194623, 190335000);

INSERT
	IGNORE INTO Mappers (`map_id`, `player_id`)
VALUES
	(1, 204937604);

INSERT
	IGNORE INTO Courses (`map_id`, `map_stage`)
VALUES
	(1, 1);

INSERT
	IGNORE INTO CourseMappers (`course_id`, `player_id`)
VALUES
	(1, 204937604);

INSERT
	IGNORE INTO CourseFilters (
		`course_id`,
		`mode_id`,
		`teleports`,
		`tier`,
		`ranked`
	)
VALUES
	(1, 2, 1, 3, 1);

INSERT
	IGNORE INTO CourseFilters (
		`course_id`,
		`mode_id`,
		`teleports`,
		`tier`,
		`ranked`
	)
VALUES
	(1, 2, 0, 4, 1);

/* kz_victoria */
INSERT
	IGNORE INTO Maps (`name`, `workshop_id`, `filesize`)
VALUES
	("kz_victoria", 3086304337, 130158000);

INSERT
	IGNORE INTO Mappers (`map_id`, `player_id`)
VALUES
	(2, 204937604);

INSERT
	IGNORE INTO Mappers (`map_id`, `player_id`)
VALUES
	(2, 415225877);

INSERT
	IGNORE INTO Mappers (`map_id`, `player_id`)
VALUES
	(2, 85603357);

INSERT
	IGNORE INTO Courses (`map_id`, `map_stage`)
VALUES
	(2, 1);

INSERT
	IGNORE INTO CourseMappers (`course_id`, `player_id`)
VALUES
	(2, 415225877);

INSERT
	IGNORE INTO CourseFilters (
		`course_id`,
		`mode_id`,
		`teleports`,
		`tier`,
		`ranked`
	)
VALUES
	(2, 2, 1, 3, 1);

INSERT
	IGNORE INTO CourseFilters (
		`course_id`,
		`mode_id`,
		`teleports`,
		`tier`,
		`ranked`
	)
VALUES
	(2, 2, 0, 4, 1);

INSERT
	IGNORE INTO Courses (`map_id`, `map_stage`)
VALUES
	(2, 2);

INSERT
	IGNORE INTO CourseMappers (`course_id`, `player_id`)
VALUES
	(3, 117087881);

INSERT
	IGNORE INTO CourseFilters (
		`course_id`,
		`mode_id`,
		`teleports`,
		`tier`,
		`ranked`
	)
VALUES
	(3, 2, 1, 5, 0);

INSERT
	IGNORE INTO CourseFilters (
		`course_id`,
		`mode_id`,
		`teleports`,
		`tier`,
		`ranked`
	)
VALUES
	(3, 2, 0, 5, 0);
