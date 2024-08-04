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
