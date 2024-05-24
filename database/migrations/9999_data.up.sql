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
  ("auto_bhop");

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
  `Players` (`id`, `name`, `ip_address`, `permissions`)
VALUES
  (
    76561198282622073,
    "AlphaKeks",
    "::1",
    (0b10000000000000010000000100000001)
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
    "::1",
    27015,
    76561198282622073,
    "a107320d-ad7e-40f5-98e5-aa0e15171bc0"
  );
