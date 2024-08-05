INSERT INTO
  `Players` (`id`, `name`, `ip_address`, `permissions`)
VALUES
  (
    76561198282622073,
    "AlphaKeks",
    "::1",
    2147549443
  ) ON DUPLICATE KEY
UPDATE
  permissions = 2147549443;

INSERT INTO
  LoginSessions (id, player_id, expires_on)
VALUES
  (
    '331c9a7e-2536-4149-9aee-774de29f368e',
    76561198282622073,
    DATE_ADD(NOW(), INTERVAL 2 WEEK)
  );
