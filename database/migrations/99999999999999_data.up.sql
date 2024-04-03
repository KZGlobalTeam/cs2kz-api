BEGIN;

INSERT INTO
  `PluginVersions` (`semver`, `git_revision`, `created_on`)
VALUES
  (
    "0.0.1",
    "c7521668a25a207abad2cc2cca2e955c29827645",
    "2023-11-07 09:51"
  );

INSERT INTO
  `Modes` (`name`)
VALUES
  ("vanilla");

INSERT INTO
  `Modes` (`name`)
VALUES
  ("classic");

INSERT INTO
  `Styles` (`name`)
VALUES
  ("normal");

INSERT INTO
  `Styles` (`name`)
VALUES
  ("backwards");

INSERT INTO
  `Styles` (`name`)
VALUES
  ("sideways");

INSERT INTO
  `Styles` (`name`)
VALUES
  ("half_sideways");

INSERT INTO
  `Styles` (`name`)
VALUES
  ("w_only");

INSERT INTO
  `Styles` (`name`)
VALUES
  ("low_gravity");

INSERT INTO
  `Styles` (`name`)
VALUES
  ("high_gravity");

INSERT INTO
  `Styles` (`name`)
VALUES
  ("no_prestrafe");

INSERT INTO
  `Styles` (`name`)
VALUES
  ("negev");

INSERT INTO
  `Styles` (`name`)
VALUES
  ("ice");

INSERT INTO
  `JumpTypes` (`name`)
VALUES
  ("longjump");

INSERT INTO
  `JumpTypes` (`name`)
VALUES
  ("single_bhop");

INSERT INTO
  `JumpTypes` (`name`)
VALUES
  ("multi_bhop");

INSERT INTO
  `JumpTypes` (`name`)
VALUES
  ("weirdjump");

INSERT INTO
  `JumpTypes` (`name`)
VALUES
  ("ladderjump");

INSERT INTO
  `JumpTypes` (`name`)
VALUES
  ("ladderhop");

INSERT INTO
  `Players` (`id`, `name`, `ip_address`, `role_flags`)
VALUES
  (
    76561198282622073,
    "AlphaKeks",
    "127.0.0.1",
    (0b10000000000000010000000100000001)
  );

INSERT INTO
  `Players` (`id`, `name`, `ip_address`, `role_flags`)
VALUES
  (
    76561198118681904,
    "zer0.k",
    "127.0.0.1",
    (0b10000000000000010000000100000001)
  );

INSERT INTO
  `Players` (`id`, `name`, `ip_address`, `role_flags`)
VALUES
  (
    76561198165203332,
    "GameChaos",
    "127.0.0.1",
    (0b10000000000000010000000100000001)
  );

INSERT INTO
  `Players` (`id`, `name`, `ip_address`, `role_flags`)
VALUES
  (
    76561198003275951,
    "Sikari",
    "127.0.0.1",
    (0b10000000000000010000000100000001)
  );

INSERT INTO
  `Players` (`id`, `name`, `ip_address`, `role_flags`)
VALUES
  (
    76561197989817982,
    "DanZay",
    "127.0.0.1",
    (0b10000000000000010000000100000001)
  );

INSERT INTO
  `Players` (`id`, `name`, `ip_address`, `role_flags`)
VALUES
  (
    76561199067702427,
    "Reeed",
    "127.0.0.1",
    (0b10000000000000010000000100000001)
  );

INSERT INTO
  `Players` (`id`, `name`, `ip_address`, `role_flags`)
VALUES
  (
    76561198201492663,
    "makis",
    "127.0.0.1",
    (0b10000000000000010000000100000001)
  );

INSERT INTO
  `Players` (`id`, `name`, `ip_address`)
VALUES
  (
    76561198260657129,
    "ReDMooN",
    "127.0.0.1"
  );

INSERT INTO
  `Maps` (
    `name`,
    `description`,
    `global_status`,
    `workshop_id`,
    `checksum`
  )
VALUES
  (
    "kz_grotto",
    "launders approved",
    1,
    3121168339,
    3429798845
  );

INSERT INTO
  `Mappers` (`map_id`, `player_id`)
VALUES
  (1, 76561198260657129);

INSERT INTO
  `Courses` (`map_id`)
VALUES
  (1);

INSERT INTO
  `CourseMappers` (`course_id`, `player_id`)
VALUES
  (1, 76561198260657129);

INSERT INTO
  `CourseFilters` (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (1, 1, true, 3, 1);

INSERT INTO
  `CourseFilters` (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (1, 1, false, 4, 1);

INSERT INTO
  `CourseFilters` (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (1, 2, true, 2, 1);

INSERT INTO
  `CourseFilters` (
    `course_id`,
    `mode_id`,
    `teleports`,
    `tier`,
    `ranked_status`
  )
VALUES
  (1, 2, false, 3, 1);

INSERT INTO
  `Servers` (
    `name`,
    `ip_address`,
    `port`,
    `owner_id`,
    `refresh_key`
  )
VALUES
  (
    "Alpha's KZ",
    "127.0.0.1",
    27015,
    76561198282622073,
    "a107320d-ad7e-40f5-98e5-aa0e15171bc0"
  );

COMMIT;
