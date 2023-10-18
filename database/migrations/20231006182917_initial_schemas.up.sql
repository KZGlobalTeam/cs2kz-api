-- Copyright (C) AlphaKeks <alphakeks@dawn.sh>
--
-- This is free software. You can redistribute it and / or modify it under the terms of the
-- GNU General Public License as published by the Free Software Foundation, either version 3
-- of the License, or (at your option) any later version.
--
-- You should have received a copy of the GNU General Public License along with this repository.
-- If not, see <https://www.gnu.org/licenses/>.
CREATE TABLE IF NOT EXISTS Players (
	-- Steam32 ID of the player
	id INT4 UNSIGNED NOT NULL,
	-- Steam username of the player
	name VARCHAR(32) NOT NULL,
	-- Whether the player is allowed to play on global servers and submit records
	is_banned BOOLEAN NOT NULL DEFAULT false,
	PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS Maps (
	id INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
	name VARCHAR(32) NOT NULL,
	-- Steam Workshop ID if the map was uploaded there
	workshop_id INT4 UNSIGNED,
	-- Size of the map's `.bsp` file
	filesize BIGINT UNSIGNED NOT NULL,
	-- The player who owns the map
	owned_by INT4 UNSIGNED NOT NULL,
	created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY (id),
	FOREIGN KEY (owned_by) REFERENCES Players(id)
);

CREATE TABLE IF NOT EXISTS Courses (
	id INT4 UNSIGNED NOT NULL AUTO_INCREMENT,
	-- The map this course belongs to
	map_id INT2 UNSIGNED NOT NULL,
	-- The stage this course represents
	--   0   => main course
	--   1.. => bonus course
	stage INT1 UNSIGNED NOT NULL,
	-- The difficulty rating of this course, on a scale of 1-10
	difficulty INT1 UNSIGNED NOT NULL,
	-- The player who mapped this course
	created_by INT4 UNSIGNED NOT NULL,
	PRIMARY KEY (id),
	FOREIGN KEY (map_id) REFERENCES Maps (id),
	FOREIGN KEY (created_by) REFERENCES Players (id),
	CONSTRAINT valid_difficulty CHECK(difficulty BETWEEN 1 AND 10)
);

CREATE TABLE IF NOT EXISTS Modes (
	id INT1 UNSIGNED NOT NULL AUTO_INCREMENT,
	name VARCHAR(16) NOT NULL,
	created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY (id),
	CONSTRAINT valid_name CHECK(name LIKE "kz_%")
);

CREATE TABLE IF NOT EXISTS Styles (
	id INT1 UNSIGNED NOT NULL AUTO_INCREMENT,
	name VARCHAR(16) NOT NULL,
	created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS Filters (
	id INT4 UNSIGNED NOT NULL,
	course_id INT4 UNSIGNED NOT NULL,
	mode_id INT1 UNSIGNED NOT NULL,
	style_id INT1 UNSIGNED NOT NULL,
	PRIMARY KEY (id),
	FOREIGN KEY (course_id) REFERENCES Courses(id),
	FOREIGN KEY (mode_id) REFERENCES Modes(id),
	FOREIGN KEY (style_id) REFERENCES Styles(id)
);

CREATE TABLE IF NOT EXISTS Servers (
	id INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
	name VARCHAR(255) NOT NULL,
	ip_address INET4 NOT NULL,
	port INT2 NOT NULL,
	-- The player who registered this server
	owned_by INT4 UNSIGNED NOT NULL,
	-- The admin who approved this server
	approved_by INT4 UNSIGNED NOT NULL,
	approved_on TIMESTAMP NOT NULL,
	api_key INT4 UNSIGNED,
	current_token INT4 UNSIGNED,
	next_token INT4 UNSIGNED,
	token_expires_at TIMESTAMP,
	token_last_used_at TIMESTAMP,
	PRIMARY KEY (id),
	FOREIGN KEY (owned_by) REFERENCES Players(id),
	FOREIGN KEY (approved_by) REFERENCES Players(id),
	CONSTRAINT valid_port CHECK(port > 0)
);

CREATE TABLE IF NOT EXISTS Playtimes (
	id INT4 UNSIGNED NOT NULL,
	player_id INT4 UNSIGNED NOT NULL,
	server_id INT2 UNSIGNED NOT NULL,
	playtime TIME NOT NULL,
	afktime TIME NOT NULL,
	plugin_version INT2 UNSIGNED NOT NULL,
	created_on DATE NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY (id),
	FOREIGN KEY (player_id) REFERENCES Players(id),
	FOREIGN KEY (server_id) REFERENCES Servers(id),
	UNIQUE (player_id, server_id, created_on)
);

CREATE TABLE IF NOT EXISTS CourseStats (
	id INT4 UNSIGNED NOT NULL,
	filter_id INT4 UNSIGNED NOT NULL,
	player_id INT4 UNSIGNED NOT NULL,
	server_id INT2 UNSIGNED NOT NULL,
	playtime TIME NOT NULL,
	attempted_runs INT2 UNSIGNED NOT NULL,
	finished_runs INT2 UNSIGNED NOT NULL,
	perfs INT2 UNSIGNED NOT NULL,
	bhops_tick0 INT2 UNSIGNED NOT NULL,
	bhops_tick1 INT2 UNSIGNED NOT NULL,
	bhops_tick2 INT2 UNSIGNED NOT NULL,
	bhops_tick3 INT2 UNSIGNED NOT NULL,
	bhops_tick4 INT2 UNSIGNED NOT NULL,
	bhops_tick5 INT2 UNSIGNED NOT NULL,
	bhops_tick6 INT2 UNSIGNED NOT NULL,
	bhops_tick7 INT2 UNSIGNED NOT NULL,
	bhops_tick8 INT2 UNSIGNED NOT NULL,
	plugin_version INT2 UNSIGNED NOT NULL,
	created_on DATE NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY (id),
	FOREIGN KEY (player_id) REFERENCES Players(id),
	FOREIGN KEY (server_id) REFERENCES Servers(id),
	UNIQUE (filter_id, player_id, server_id, created_on)
);

-- Records the Anti-Cheat has determined to be "legit".
CREATE TABLE IF NOT EXISTS Records (
	id INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
	filter_id INT4 UNSIGNED NOT NULL,
	player_id INT4 UNSIGNED NOT NULL,
	server_id INT2 UNSIGNED NOT NULL,
	teleports INT2 UNSIGNED NOT NULL,
	time FLOAT8 NOT NULL,
	plugin_version INT2 UNSIGNED NOT NULL,
	created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY (id),
	FOREIGN KEY (filter_id) REFERENCES Filters(id),
	FOREIGN KEY (player_id) REFERENCES Players(id),
	FOREIGN KEY (server_id) REFERENCES Servers(id)
);

-- Records the Anti-Cheat has determined to require manual verification.
CREATE TABLE IF NOT EXISTS RecordsToCheck (
	id INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
	course_id INT4 UNSIGNED NOT NULL,
	mode_id INT1 UNSIGNED NOT NULL,
	style_id INT1 UNSIGNED NOT NULL,
	player_id INT4 UNSIGNED NOT NULL,
	server_id INT2 UNSIGNED NOT NULL,
	teleports INT2 UNSIGNED NOT NULL,
	-- How many ingame ticks passed during this run
	ticks INT4 UNSIGNED NOT NULL,
	plugin_version INT2 UNSIGNED NOT NULL,
	created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY (id),
	FOREIGN KEY (course_id) REFERENCES Courses(id),
	FOREIGN KEY (mode_id) REFERENCES Modes(id),
	FOREIGN KEY (style_id) REFERENCES Styles(id),
	FOREIGN KEY (player_id) REFERENCES Players(id),
	FOREIGN KEY (server_id) REFERENCES Servers(id)
);

-- Records the Anti-Cheat has determined to be "cheated".
CREATE TABLE IF NOT EXISTS RecordsCheated (
	id INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
	course_id INT4 UNSIGNED NOT NULL,
	mode_id INT1 UNSIGNED NOT NULL,
	style_id INT1 UNSIGNED NOT NULL,
	player_id INT4 UNSIGNED NOT NULL,
	server_id INT2 UNSIGNED NOT NULL,
	teleports INT2 UNSIGNED NOT NULL,
	-- How many ingame ticks passed during this run
	ticks INT4 UNSIGNED NOT NULL,
	plugin_version INT2 UNSIGNED NOT NULL,
	created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY (id),
	FOREIGN KEY (course_id) REFERENCES Courses(id),
	FOREIGN KEY (mode_id) REFERENCES Modes(id),
	FOREIGN KEY (style_id) REFERENCES Styles(id),
	FOREIGN KEY (player_id) REFERENCES Players(id),
	FOREIGN KEY (server_id) REFERENCES Servers(id)
);

CREATE TABLE IF NOT EXISTS JumpstatsTypes (
	id INT1 UNSIGNED NOT NULL AUTO_INCREMENT,
	name VARCHAR(16) NOT NULL,
	created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS Jumpstats (
	id INT4 UNSIGNED NOT NULL AUTO_INCREMENT,
	`type` INT1 UNSIGNED NOT NULL,
	distance FLOAT8 NOT NULL,
	mode_id INT1 UNSIGNED NOT NULL,
	style_id INT1 UNSIGNED NOT NULL,
	player_id INT4 UNSIGNED NOT NULL,
	server_id INT2 UNSIGNED NOT NULL,
	plugin_version INT2 UNSIGNED NOT NULL,
	created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY (id),
	FOREIGN KEY (`type`) REFERENCES JumpstatsTypes(id),
	FOREIGN KEY (mode_id) REFERENCES Modes(id),
	FOREIGN KEY (style_id) REFERENCES Styles(id),
	FOREIGN KEY (player_id) REFERENCES Players(id),
	FOREIGN KEY (server_id) REFERENCES Servers(id)
);

CREATE TABLE IF NOT EXISTS Bans (
	id INT4 UNSIGNED NOT NULL AUTO_INCREMENT,
	player_id INT4 UNSIGNED NOT NULL,
	-- Will be NULL if the player was banned by the Anti-Cheat or by an admin directly
	player_ip INET4,
	-- Will be NULL if the player was banned by the Anti-Cheat or by an admin directly
	server_id INT2 UNSIGNED,
	reason VARCHAR(2048) NOT NULL,
	-- Will be NULL if the player was banned by the Anti-Cheat
	banned_by INT4 UNSIGNED,
	plugin_version INT2 UNSIGNED NOT NULL,
	created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	expires_on TIMESTAMP,
	PRIMARY KEY (id),
	FOREIGN KEY (player_id) REFERENCES Players(id),
	FOREIGN KEY (server_id) REFERENCES Servers(id),
	FOREIGN KEY (banned_by) REFERENCES Players(id)
);

CREATE TABLE IF NOT EXISTS Unbans (
	id INT4 UNSIGNED NOT NULL AUTO_INCREMENT,
	ban_id INT4 UNSIGNED NOT NULL,
	player_id INT4 UNSIGNED NOT NULL,
	reason VARCHAR(2048),
	-- The admin who lifted this ban
	unbanned_by INT4 UNSIGNED NOT NULL,
	created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY (id),
	FOREIGN KEY (ban_id) REFERENCES Bans(id),
	FOREIGN KEY (player_id) REFERENCES Players(id),
	FOREIGN KEY (unbanned_by) REFERENCES Players(id)
);

CREATE TABLE IF NOT EXISTS PluginVersions (
	id INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
	semver VARCHAR(14) NOT NULL,
	git_rev VARCHAR(255) NOT NULL,
	created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY (id)
);
