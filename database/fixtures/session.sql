INSERT
  IGNORE INTO Players (id, name, ip_address)
VALUES
  (76561197960265729, 'fake', '::1');

INSERT INTO
  LoginSessions (id, player_id, expires_on)
VALUES
  (
    '331c9a7e-2536-4149-9aee-774de29f368e',
    76561197960265729,
    DATE_ADD(NOW(), INTERVAL 2 WEEK)
  );
