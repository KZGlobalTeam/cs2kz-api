INSERT INTO
  `PluginVersions` (`semver`, `git_revision`, `created_on`)
VALUES
  (
    "0.0.1",
    "c7521668a25a207abad2cc2cca2e955c29827645",
    "2023-11-07 09:51"
  );

INSERT INTO
  `Players` (`id`, `name`, `ip_address`, `permissions`)
VALUES
  (
    76561198282622073,
    "AlphaKeks",
    "::1",
    2147549441
  );

INSERT INTO
  `Servers` (
    `name`,
    `host`,
    `port`,
    `owner_id`,
    `key`
  )
VALUES
  (
    "Alpha's KZ",
    "::1",
    27015,
    76561198282622073,
    "a107320d-ad7e-40f5-98e5-aa0e15171bc0"
  );
