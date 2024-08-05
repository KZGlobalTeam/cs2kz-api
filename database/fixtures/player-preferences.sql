INSERT INTO
  `Players` (`id`, `name`, `ip_address`, `preferences`)
VALUES
  (
    76561198282622073,
    "AlphaKeks",
    "::1",
    '{"foo":"bar"}'
  ) ON DUPLICATE KEY
UPDATE
  preferences = '{"foo":"bar"}';
