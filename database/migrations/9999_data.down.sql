DELETE FROM
  `Servers`
WHERE
  `id` = 1;

IF (
  SELECT
    COUNT(`id`)
  FROM
    `Servers`
) = 0 THEN
ALTER TABLE
  `Servers` AUTO_INCREMENT = 1;

END IF;

DELETE FROM
  `Players`
WHERE
  `id` = 76561198282622073;

DELETE FROM
  `JumpTypes`
WHERE
  `id` <= 6;

IF (
  SELECT
    COUNT(`id`)
  FROM
    `JumpTypes`
) = 0 THEN
ALTER TABLE
  `JumpTypes` AUTO_INCREMENT = 1;

END IF;

DELETE FROM
  `Styles`
WHERE
  `id` <= 2;

IF (
  SELECT
    COUNT(`id`)
  FROM
    `Styles`
) = 0 THEN
ALTER TABLE
  `Styles` AUTO_INCREMENT = 1;

END IF;

DELETE FROM
  `Modes`
WHERE
  `id` <= 2;

IF (
  SELECT
    COUNT(`id`)
  FROM
    `Modes`
) = 0 THEN
ALTER TABLE
  `Modes` AUTO_INCREMENT = 1;

END IF;

DELETE FROM
  `PluginVersions`
WHERE
  `id` = 1;

IF (
  SELECT
    COUNT(`id`)
  FROM
    `PluginVersions`
) = 0 THEN
ALTER TABLE
  `PluginVersions` AUTO_INCREMENT = 1;

END IF;
