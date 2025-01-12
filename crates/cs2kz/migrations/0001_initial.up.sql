CREATE TABLE IF NOT EXISTS PluginVersions (
  id INT2 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  major INT8 UNSIGNED NOT NULL,
  minor INT8 UNSIGNED NOT NULL,
  patch INT8 UNSIGNED NOT NULL,
  pre VARCHAR(255) NOT NULL,
  build VARCHAR(255) NOT NULL,
  git_revision BINARY(20) NOT NULL UNIQUE,
  published_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT UC_semver UNIQUE (major, minor, patch, pre, build)
);

CREATE TABLE IF NOT EXISTS AccessKeys (
  name VARCHAR(255) NOT NULL PRIMARY KEY,
  value BINARY(16) NOT NULL UNIQUE,
  expires_at TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS Users (
  id INT8 UNSIGNED NOT NULL PRIMARY KEY,
  name VARCHAR(255) NOT NULL,
  email_address VARCHAR(255) UNIQUE,
  -- see `cs2kz::users::Permissions` struct in the Rust code
  permissions INT8 UNSIGNED NOT NULL DEFAULT 0,
  registered_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_login_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS UserSessions (
  id BINARY(16) NOT NULL PRIMARY KEY,
  user_id INT8 UNSIGNED NOT NULL REFERENCES Users(id) ON DELETE CASCADE,
  expires_at TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS Servers (
  id INT2 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  name VARCHAR(255) NOT NULL UNIQUE,
  host VARCHAR(255) NOT NULL,
  port INT2 UNSIGNED NOT NULL,
  owner_id INT8 UNSIGNED NOT NULL REFERENCES Users(id),
  access_key BINARY(16) UNIQUE,
  approved_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_connected_at TIMESTAMP,
  CONSTRAINT UC_host_port UNIQUE (host, port)
);

CREATE TABLE IF NOT EXISTS Players (
  id INT8 UNSIGNED NOT NULL PRIMARY KEY,
  name VARCHAR(255) NOT NULL,
  ip_address INET4,
  preferences JSON NOT NULL DEFAULT '{}',
  first_joined_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_joined_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS Maps (
  id INT2 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  workshop_id INT4 UNSIGNED NOT NULL,
  name VARCHAR(255) NOT NULL,
  description TEXT,
  -- see `cs2kz::maps::MapState` enum in the Rust code
  state INT1 NOT NULL DEFAULT -1,
  vpk_checksum BINARY(16) NOT NULL,
  approved_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS Courses (
  id INT2 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  map_id INT2 UNSIGNED NOT NULL REFERENCES Maps(id) ON DELETE CASCADE,
  name VARCHAR(255) NOT NULL,
  description TEXT,
  CONSTRAINT UC_name_per_map UNIQUE (map_id, name)
);

CREATE TABLE IF NOT EXISTS CourseFilters (
  id INT2 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  course_id INT2 UNSIGNED NOT NULL REFERENCES Courses(id) ON DELETE CASCADE,
  -- see `cs2kz::Mode` enum in the Rust code
  `mode` INT1 UNSIGNED NOT NULL,
  -- see `cs2kz::maps::courses::filters::Tier` enum in the Rust code
  nub_tier INT1 UNSIGNED NOT NULL CHECK (nub_tier BETWEEN 1 AND 10),
  -- see `cs2kz::maps::courses::filters::Tier` enum in the Rust code
  pro_tier INT1 UNSIGNED NOT NULL CHECK (pro_tier BETWEEN 1 AND 10),
  -- see `cs2kz::maps::courses::filters::CourseFilterState` enum in the Rust code
  state INT1 NOT NULL DEFAULT -1,
  notes TEXT,
  CONSTRAINT UC_mode_per_course UNIQUE (course_id, `mode`)
);

CREATE TABLE IF NOT EXISTS Mappers (
  map_id INT2 UNSIGNED NOT NULL REFERENCES Maps(id) ON DELETE CASCADE,
  player_id INT8 UNSIGNED NOT NULL REFERENCES Players(id),
  PRIMARY KEY (map_id, player_id)
);

CREATE TABLE IF NOT EXISTS CourseMappers (
  course_id INT2 UNSIGNED NOT NULL REFERENCES Courses(id) ON DELETE CASCADE,
  player_id INT8 UNSIGNED NOT NULL REFERENCES Players(id),
  PRIMARY KEY (course_id, player_id)
);

CREATE TABLE IF NOT EXISTS Jumps (
  id INT4 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  player_id INT8 UNSIGNED NOT NULL REFERENCES Players(id),
  server_id INT2 UNSIGNED NOT NULL REFERENCES Servers(id),
  -- see `cs2kz::Mode` enum in the Rust code
  `mode` INT1 UNSIGNED NOT NULL,
  -- see `cs2kz::Styles` struct in the Rust code
  styles INT4 UNSIGNED NOT NULL,
  -- see `cs2kz::JumpType` struct in the Rust code
  `type` INT1 UNSIGNED NOT NULL,
  time FLOAT8 NOT NULL CHECK (time > 0),
  strafes INT1 UNSIGNED NOT NULL,
  distance FLOAT4 NOT NULL,
  sync FLOAT4 NOT NULL,
  pre FLOAT4 NOT NULL,
  max FLOAT4 NOT NULL,
  overlap FLOAT4 NOT NULL,
  bad_angles FLOAT4 NOT NULL,
  dead_air FLOAT4 NOT NULL,
  height FLOAT4 NOT NULL,
  airpath FLOAT4 NOT NULL,
  deviation FLOAT4 NOT NULL,
  average_width FLOAT4 NOT NULL,
  plugin_version_id INT2 UNSIGNED NOT NULL REFERENCES PluginVersions(id),
  submitted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS JumpReplays (
  jump_id INT4 UNSIGNED NOT NULL PRIMARY KEY REFERENCES Jumps(id) ON DELETE CASCADE,
  data BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS Records (
  id INT4 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  player_id INT8 UNSIGNED NOT NULL REFERENCES Players(id),
  server_id INT2 UNSIGNED NOT NULL REFERENCES Servers(id),
  filter_id INT2 UNSIGNED NOT NULL REFERENCES CourseFilters(id),
  -- see `cs2kz::Styles` struct in the Rust code
  styles INT4 UNSIGNED NOT NULL,
  teleports INT4 UNSIGNED NOT NULL,
  time FLOAT8 NOT NULL CHECK (time > 0),
  plugin_version_id INT2 UNSIGNED NOT NULL REFERENCES PluginVersions(id),
  submitted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS RecordReplays (
  record_id INT4 UNSIGNED NOT NULL PRIMARY KEY REFERENCES Records(id) ON DELETE CASCADE,
  data BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS BestNubRecords (
  filter_id INT2 UNSIGNED NOT NULL REFERENCES CourseFilters(id),
  player_id INT8 UNSIGNED NOT NULL REFERENCES Players(id),
  record_id INT4 UNSIGNED NOT NULL REFERENCES Records(id) ON DELETE CASCADE,
  points FLOAT8 NOT NULL,
  PRIMARY KEY (filter_id, player_id)
);

CREATE TABLE IF NOT EXISTS BestProRecords (
  filter_id INT2 UNSIGNED NOT NULL REFERENCES CourseFilters(id),
  player_id INT8 UNSIGNED NOT NULL REFERENCES Players(id),
  record_id INT4 UNSIGNED NOT NULL REFERENCES Records(id) ON DELETE CASCADE,
  points FLOAT8 NOT NULL,
  points_based_on_pro_leaderboard BOOLEAN NOT NULL,
  PRIMARY KEY (filter_id, player_id)
);

CREATE TABLE IF NOT EXISTS PointDistributionData (
  filter_id INT2 UNSIGNED NOT NULL REFERENCES CourseFilters(id),
  is_pro_leaderboard BOOLEAN NOT NULL,
  a FLOAT8 NOT NULL,
  b FLOAT8 NOT NULL,
  loc FLOAT8 NOT NULL,
  scale FLOAT8 NOT NULL,
  top_scale FLOAT8 NOT NULL,
  PRIMARY KEY (filter_id, is_pro_leaderboard)
);

CREATE TABLE IF NOT EXISTS RecordCounts (
  filter_id INT2 UNSIGNED NOT NULL PRIMARY KEY REFERENCES CourseFilters(id),
  count INT8 UNSIGNED NOT NULL
);

CREATE TABLE IF NOT EXISTS FiltersToRecalculate (
  filter_id INT2 UNSIGNED NOT NULL PRIMARY KEY REFERENCES CourseFilters(id)
);

CREATE TABLE IF NOT EXISTS Bans (
  id INT4 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  player_id INT8 UNSIGNED NOT NULL REFERENCES Players(id),
  player_ip INET4,
  -- see `cs2kz::bans::BannedBy` enum in the Rust code
  banned_by INT8 UNSIGNED NOT NULL,
  reason VARCHAR(255) NOT NULL,
  plugin_version_id INT2 UNSIGNED NOT NULL REFERENCES PluginVersions(id),
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  expires_at TIMESTAMP NOT NULL CHECK (expires_at >= created_at)
);

CREATE TABLE IF NOT EXISTS Unbans (
  ban_id INT4 UNSIGNED NOT NULL PRIMARY KEY REFERENCES Bans(id),
  admin_id INT8 UNSIGNED NOT NULL REFERENCES Users(id),
  reason VARCHAR(255) NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE
OR REPLACE FUNCTION KZ_POINTS(
  tier INT1 UNSIGNED,
  is_pro_leaderboard BOOLEAN,
  rank INT4 UNSIGNED,
  dist_points FLOAT8
) RETURNS FLOAT8
BEGIN
DECLARE
for_tier,
remaining,
for_rank FLOAT8;

SET
  for_tier = CASE
    tier
    WHEN 2 THEN 500
    WHEN 3 THEN 2000
    WHEN 4 THEN 3500
    WHEN 5 THEN 5000
    WHEN 6 THEN 6500
    WHEN 7 THEN 8000
    WHEN 8 THEN 9500
  END;

IF (is_pro_leaderboard) THEN
SET
  for_tier = for_tier + (10000 - for_tier) * 0.1;

END IF;

SET
  remaining = 10000 - for_tier;

SET
  for_rank = 0;

IF (rank < 100) THEN
SET
  for_rank = (100 - rank) * 0.004;

END IF;

IF (rank < 20) THEN
SET
  for_rank = for_rank + (20 - rank) * 0.02;

END IF;

SET
  for_rank = for_rank + (
    CASE
      rank
      WHEN 0 THEN 0.2
      WHEN 1 THEN 0.12
      WHEN 2 THEN 0.09
      WHEN 3 THEN 0.06
      WHEN 4 THEN 0.02
    END
  );

RETURN for_tier + (0.125 * remaining * for_rank) + (0.875 * remaining * dist_points);

END;
