INSERT INTO
	Players (`steam_id`, `name`, `last_known_ip_address`)
VALUES
	(322356345, "AlphaKeks", "127.0.0.1");

INSERT INTO
	Players (`steam_id`, `name`, `last_known_ip_address`)
VALUES
	(158416176, "zer0.k", "127.0.0.1");

INSERT INTO
	Players (`steam_id`, `name`, `last_known_ip_address`)
VALUES
	(204937604, "GameChaos", "127.0.0.1");

INSERT INTO
	Players (`steam_id`, `name`, `last_known_ip_address`)
VALUES
	(43010223, "Sikari", "127.0.0.1");

INSERT INTO
	Servers (
		`name`,
		`ip_address`,
		`port`,
		`owned_by`,
		`api_key`
	)
VALUES
	(
		"Alpha's KZ",
		"127.0.0.1",
		"1337",
		322356345,
		322356345
	);

INSERT INTO
	Sessions (
		`player_id`,
		`server_id`,
		`time_active`,
		`time_spectating`,
		`time_afk`,
		`perfs`,
		`bhops_tick0`,
		`bhops_tick1`,
		`bhops_tick2`,
		`bhops_tick3`,
		`bhops_tick4`,
		`bhops_tick5`,
		`bhops_tick6`,
		`bhops_tick7`,
		`bhops_tick8`
	)
VALUES
	(
		322356345,
		1,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0
	);

INSERT INTO
	Sessions (
		`player_id`,
		`server_id`,
		`time_active`,
		`time_spectating`,
		`time_afk`,
		`perfs`,
		`bhops_tick0`,
		`bhops_tick1`,
		`bhops_tick2`,
		`bhops_tick3`,
		`bhops_tick4`,
		`bhops_tick5`,
		`bhops_tick6`,
		`bhops_tick7`,
		`bhops_tick8`
	)
VALUES
	(
		158416176,
		1,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0
	);

INSERT INTO
	Sessions (
		`player_id`,
		`server_id`,
		`time_active`,
		`time_spectating`,
		`time_afk`,
		`perfs`,
		`bhops_tick0`,
		`bhops_tick1`,
		`bhops_tick2`,
		`bhops_tick3`,
		`bhops_tick4`,
		`bhops_tick5`,
		`bhops_tick6`,
		`bhops_tick7`,
		`bhops_tick8`
	)
VALUES
	(
		204937604,
		1,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0
	);

INSERT INTO
	Sessions (
		`player_id`,
		`server_id`,
		`time_active`,
		`time_spectating`,
		`time_afk`,
		`perfs`,
		`bhops_tick0`,
		`bhops_tick1`,
		`bhops_tick2`,
		`bhops_tick3`,
		`bhops_tick4`,
		`bhops_tick5`,
		`bhops_tick6`,
		`bhops_tick7`,
		`bhops_tick8`
	)
VALUES
	(
		43010223,
		1,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0,
		0
	);

INSERT INTO
	Maps (`name`, `workshop_id`, `filesize`)
VALUES
	("kz_checkmate", 3070194623, 190335000);

INSERT INTO
	Mappers (`map_id`, `player_id`)
VALUES
	(1, 204937604);

INSERT INTO
	Courses (`map_id`, `map_stage`)
VALUES
	(1, 0);

INSERT INTO
	CourseMappers (`course_id`, `player_id`)
VALUES
	(1, 204937604);

INSERT INTO
	CourseFilters (
		`course_id`,
		`mode_id`,
		`has_teleports`,
		`tier`,
		`ranked`
	)
VALUES
	(1, 2, 1, 3, 1);

INSERT INTO
	CourseFilters (
		`course_id`,
		`mode_id`,
		`has_teleports`,
		`tier`,
		`ranked`
	)
VALUES
	(1, 2, 0, 4, 1);
