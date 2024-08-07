INSERT
  IGNORE INTO `Players` (`id`, `name`, `ip_address`)
VALUES
  (
    76561198264939817,
    "iBrahizy",
    "::1"
  );

INSERT
  IGNORE INTO `Players` (`id`, `name`, `ip_address`)
VALUES
  (
    76561198118681904,
    "zer0.k",
    "::1"
  );

INSERT INTO
  Bans (
    player_id,
    player_ip,
    reason,
    admin_id,
    plugin_version_id
  )
VALUES
  (
    76561198264939817,
    "::1",
    "auto_bhop",
    76561198282622073,
    1
  );

INSERT INTO
  Bans (
    player_id,
    player_ip,
    reason,
    admin_id,
    plugin_version_id
  )
VALUES
  (
    76561198118681904,
    "::1",
    "auto_strafe",
    76561198282622073,
    1
  );

INSERT INTO
  Bans (
    player_id,
    player_ip,
    server_id,
    reason,
    plugin_version_id
  )
VALUES
  (
    76561198282622073,
    "::1",
    1,
    "macro",
    1
  );
