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
  (304674089, "iBrahizy", "127.0.0.1");

INSERT INTO
  Players (`steam_id`, `name`, `last_known_ip_address`)
VALUES
  (43010223, "Sikari", "127.0.0.1");

INSERT INTO
  Players (`steam_id`, `name`, `last_known_ip_address`)
VALUES
  (415225877, "lars", "127.0.0.1");

INSERT INTO
  Players (`steam_id`, `name`, `last_known_ip_address`)
VALUES
  (85603357, "mark", "127.0.0.1");

INSERT INTO
  Players (`steam_id`, `name`, `last_known_ip_address`)
VALUES
  (117087881, "Kiwi", "127.0.0.1");

INSERT INTO
  Players(`steam_id`, `name`, `last_known_ip_address`)
VALUES
  (237797161, "Dima", "127.0.0.1");

INSERT INTO
  Players(`steam_id`, `name`, `last_known_ip_address`)
VALUES
  (193574091, "Fob", "127.0.0.1");

INSERT INTO
  Players(`steam_id`, `name`, `last_known_ip_address`)
VALUES
  (300391401, "ReDMooN", "127.0.0.1");

INSERT INTO
  Players(`steam_id`, `name`, `last_known_ip_address`)
VALUES
  (365313220, "smieszneznaczki", "127.0.0.1");

INSERT INTO
  Players(`steam_id`, `name`, `last_known_ip_address`)
VALUES
  (62941379, "Phinx", "127.0.0.1");

INSERT INTO
  Players(`steam_id`, `name`, `last_known_ip_address`)
VALUES
  (321627999, "SHEESHYM", "127.0.0.1");

INSERT INTO
  Players(`steam_id`, `name`, `last_known_ip_address`)
VALUES
  (122638963, "Useless S. Grant", "127.0.0.1");

INSERT INTO
  Players(`steam_id`, `name`, `last_known_ip_address`)
VALUES
  (234537517, "neon", "127.0.0.1");

INSERT INTO
  Players(`steam_id`, `name`, `last_known_ip_address`)
VALUES
  (1107436699, "Reeed", "127.0.0.1");

INSERT INTO
  Maps (`name`, `workshop_id`, `checksum`, `global_status`)
VALUES
  ("kz_checkmate", 3070194623, 133994000, 1);

INSERT INTO
  Maps (`name`, `workshop_id`, `checksum`, `global_status`)
VALUES
  ("kz_chrimstmas", 2903326571, 42070000, 1);

INSERT INTO
  Maps (`name`, `workshop_id`, `checksum`, `global_status`)
VALUES
  ("kz_phamous", 3104579274, 74697000, 1);

INSERT INTO
  Maps (`name`, `workshop_id`, `checksum`, `global_status`)
VALUES
  ("kz_ggsh", 3072744536, 31237000, 1);

INSERT INTO
  Maps (`name`, `workshop_id`, `checksum`, `global_status`, `description`)
VALUES
  (
    "kz_victoria",
    3086304337,
    130158000,
    1,
    "this map has a funny surf"
  );

INSERT INTO
  Maps (`name`, `workshop_id`, `checksum`, `global_status`, `description`)
VALUES
  (
    "kz_generic",
    3070316567,
    134684000,
    1,
    "i used to have wr on this you know"
  );

INSERT INTO
  Maps (`name`, `workshop_id`, `checksum`, `global_status`)
VALUES
  ("kz_grotto", 3121168339, 80401000, 1);

INSERT INTO
  Maps (`name`, `workshop_id`, `checksum`, `global_status`)
VALUES
  ("kz_igneous", 3102712799, 267639000, 1);

INSERT INTO
  Mappers (`map_id`, `player_id`)
VALUES
  (1, 204937604);

INSERT INTO
  Mappers (`map_id`, `player_id`)
VALUES
  (2, 300391401);

INSERT INTO
  Mappers (`map_id`, `player_id`)
VALUES
  (3, 204937604);

INSERT INTO
  Mappers (`map_id`, `player_id`)
VALUES
  (3, 62941379);

INSERT INTO
  Mappers (`map_id`, `player_id`)
VALUES
  (4, 321627999);

INSERT INTO
  Mappers (`map_id`, `player_id`)
VALUES
  (5, 204937604);

INSERT INTO
  Mappers (`map_id`, `player_id`)
VALUES
  (5, 415225877);

INSERT INTO
  Mappers (`map_id`, `player_id`)
VALUES
  (5, 85603357);

INSERT INTO
  Mappers (`map_id`, `player_id`)
VALUES
  (6, 122638963);

INSERT INTO
  Mappers (`map_id`, `player_id`)
VALUES
  (7, 300391401);

INSERT INTO
  Mappers (`map_id`, `player_id`)
VALUES
  (8, 234537517);

INSERT INTO
  Courses (`map_id`, `map_stage`)
VALUES
  (1, 1);

INSERT INTO
  Courses (`map_id`, `map_stage`)
VALUES
  (2, 1);

INSERT INTO
  Courses (`map_id`, `map_stage`, `description`)
VALUES
  (3, 1, "this course is very fun");

INSERT INTO
  Courses (`map_id`, `map_stage`)
VALUES
  (4, 1);

INSERT INTO
  Courses (`map_id`, `map_stage`)
VALUES
  (5, 1);

INSERT INTO
  Courses (`map_id`, `map_stage`)
VALUES
  (5, 2);

INSERT INTO
  Courses (`map_id`, `map_stage`)
VALUES
  (6, 1);

INSERT INTO
  Courses (`map_id`, `map_stage`)
VALUES
  (6, 2);

INSERT INTO
  Courses (`map_id`, `map_stage`)
VALUES
  (6, 3);

INSERT INTO
  Courses (`map_id`, `map_stage`)
VALUES
  (7, 1);

INSERT INTO
  Courses (`map_id`, `map_stage`)
VALUES
  (7, 2);

INSERT INTO
  Courses (`map_id`, `map_stage`)
VALUES
  (8, 1);

INSERT INTO
  CourseMappers (`course_id`, `player_id`)
VALUES
  (1, 204937604);

INSERT INTO
  CourseMappers (`course_id`, `player_id`)
VALUES
  (2, 300391401);

INSERT INTO
  CourseMappers (`course_id`, `player_id`)
VALUES
  (3, 62941379);

INSERT INTO
  CourseMappers (`course_id`, `player_id`)
VALUES
  (3, 204937604);

INSERT INTO
  CourseMappers (`course_id`, `player_id`)
VALUES
  (4, 321627999);

INSERT INTO
  CourseMappers (`course_id`, `player_id`)
VALUES
  (5, 415225877);

INSERT INTO
  CourseMappers (`course_id`, `player_id`)
VALUES
  (5, 85603357);

INSERT INTO
  CourseMappers (`course_id`, `player_id`)
VALUES
  (5, 204937604);

INSERT INTO
  CourseMappers (`course_id`, `player_id`)
VALUES
  (6, 415225877);

INSERT INTO
  CourseMappers (`course_id`, `player_id`)
VALUES
  (7, 122638963);

INSERT INTO
  CourseMappers (`course_id`, `player_id`)
VALUES
  (8, 122638963);

INSERT INTO
  CourseMappers (`course_id`, `player_id`)
VALUES
  (9, 122638963);

INSERT INTO
  CourseMappers (`course_id`, `player_id`)
VALUES
  (10, 300391401);

INSERT INTO
  CourseMappers (`course_id`, `player_id`)
VALUES
  (11, 300391401);

INSERT INTO
  CourseMappers (`course_id`, `player_id`)
VALUES
  (12, 234537517);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`,
    `notes`
  )
VALUES
  (1, 1, TRUE, 6, 1, "this is a free wr");

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (1, 1, FALSE, 7, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (1, 2, TRUE, 3, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (1, 2, FALSE, 4, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (2, 1, TRUE, 6, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (2, 1, FALSE, 7, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (2, 2, TRUE, 3, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (2, 2, FALSE, 4, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (3, 1, TRUE, 4, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (3, 1, FALSE, 5, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (3, 2, TRUE, 3, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (3, 2, FALSE, 4, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (4, 1, TRUE, 10, -1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (4, 1, FALSE, 10, -1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (4, 2, TRUE, 5, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (4, 2, FALSE, 6, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (5, 1, TRUE, 10, -1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (5, 1, FALSE, 10, -1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (5, 2, TRUE, 3, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (5, 2, FALSE, 4, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (6, 1, TRUE, 3, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (6, 1, FALSE, 4, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (6, 2, TRUE, 3, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (6, 2, FALSE, 4, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (7, 1, TRUE, 3, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (7, 1, FALSE, 4, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (7, 2, TRUE, 3, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (7, 2, FALSE, 4, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (8, 1, TRUE, 10, -1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (8, 1, FALSE, 10, -1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (8, 2, TRUE, 2, -1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (8, 2, FALSE, 2, -1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (9, 1, TRUE, 10, -1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (9, 1, FALSE, 10, -1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (9, 2, TRUE, 3, -1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (9, 2, FALSE, 3, -1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (10, 1, TRUE, 4, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (10, 1, FALSE, 5, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (10, 2, TRUE, 3, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (10, 2, FALSE, 4, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (11, 1, TRUE, 3, -1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (11, 1, FALSE, 4, -1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (11, 2, TRUE, 2, -1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (11, 2, FALSE, 3, -1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (12, 1, TRUE, 10, -1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (12, 1, FALSE, 10, -1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (12, 2, TRUE, 3, 1);

INSERT INTO
  CourseFilters (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (12, 2, FALSE, 4, 1);

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
    "27015",
    322356345,
    4389274
  );

INSERT INTO
  PluginVersions (`version`, `revision`)
VALUES
  (
    "0.0.1",
    "58c1ef12c94d6f740acd9a5f3a85acc1b48e613c"
  );

INSERT INTO
  Admins (`steam_id`, `role_flags`)
VALUES
  (
    322356345,
    1 << 0 | 1 << 8 | 1 << 16 | 1 << 31
  );

INSERT INTO
  Admins (`steam_id`, `role_flags`)
VALUES
  (
    1107436699,
    1 << 0 | 1 << 8 | 1 << 16 | 1 << 31
  );
