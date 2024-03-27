BEGIN;

INSERT INTO
  `PluginVersions` (`semver`, `git_revision`)
VALUES
  (
    "0.0.0",
    "caffc305d3e03b9a21457e16303a9dedf8ef87ed"
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
    (0b10000000000000000000000000000000)
  );

INSERT INTO
  `Players` (`id`, `name`, `ip_address`, `role_flags`)
VALUES
  (
    76561198118681904,
    "zer0.k",
    "127.0.0.1",
    (0b10000000000000000000000000000000)
  );

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
