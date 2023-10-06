CREATE TABLE Players (
	-- Steam32 ID
	id INT4 UNSIGNED NOT NULL,

	name VARCHAR(32) NOT NULL,
	is_banned BOOLEAN NOT NULL,

	-- Total amount of seconds spent on verified KZ servers.
	playtime INT4 UNSIGNED NOT NULL,

	PRIMARY KEY (id)
);

CREATE TABLE Modes (
	id INT1 UNSIGNED NOT NULL AUTO_INCREMENT,
	name VARCHAR(255) NOT NULL,
	created_on DATETIME NOT NULL,

	PRIMARY KEY (id)
);

CREATE TABLE Styles (
	id INT1 UNSIGNED NOT NULL AUTO_INCREMENT,
	name VARCHAR(255) NOT NULL,
	created_on DATETIME NOT NULL,

	PRIMARY KEY (id)
);

CREATE TABLE Maps (
	id INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
	name VARCHAR(32) NOT NULL,

	-- Steam Workshop ID if the map was uploaded there.
	workshop_id INT UNSIGNED,
	created_on DATETIME NOT NULL,

	PRIMARY KEY (id)
);

CREATE TABLE Courses (
	id INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
	map_id INT2 UNSIGNED NOT NULL,
	stage INT1 UNSIGNED NOT NULL,
	difficulty INT1 UNSIGNED NOT NULL,
	creator_id INT4 UNSIGNED NOT NULL,

	PRIMARY KEY (id),
	FOREIGN KEY (map_id) REFERENCES Maps(id),
	FOREIGN KEY (creator_id) REFERENCES Players(id),

	CONSTRAINT valid_difficulty
		 CHECK (difficulty BETWEEN 1 AND 10)
);

CREATE TABLE Filters (
	course_id INT2 UNSIGNED NOT NULL,
	mode_id INT1 UNSIGNED NOT NULL,

	PRIMARY KEY (course_id, mode_id),

	FOREIGN KEY (course_id) REFERENCES Courses(id),
	FOREIGN KEY (mode_id) REFERENCES Modes(id)
);

CREATE TABLE Servers (
	id INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
	name VARCHAR(255) NOT NULL,
	ip_address INET4 NOT NULL,
	port INT2 NOT NULL,
	owner_id INT4 UNSIGNED NOT NULL,
	approved_by INT4 UNSIGNED NOT NULL,
	approved_on DATETIME NOT NULL,

	PRIMARY KEY (id),
	FOREIGN KEY (owner_id) REFERENCES Players(id),
	FOREIGN KEY (approved_by) REFERENCES Players(id),
	CONSTRAINT valid_port CHECK(port BETWEEN 1 AND 65535)
);

-- Records the Anti-Cheat has determined to be "legit".
CREATE TABLE Records (
	id INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
	course_id INT2 UNSIGNED NOT NULL,
	mode_id INT1 UNSIGNED NOT NULL,
	style_id INT1 UNSIGNED NOT NULL,
	player_id INT4 UNSIGNED NOT NULL,
	server_id INT2 UNSIGNED NOT NULL,
	teleports INT2 UNSIGNED NOT NULL,

	-- Amount of ingame ticks it took to complete this run.
	ticks INT4 UNSIGNED NOT NULL,

	created_on DATETIME NOT NULL,

	PRIMARY KEY (id),
	FOREIGN KEY (course_id) REFERENCES Courses(id),
	FOREIGN KEY (mode_id) REFERENCES Modes(id),
	FOREIGN KEY (style_id) REFERENCES Styles(id),
	FOREIGN KEY (player_id) REFERENCES Players(id),
	FOREIGN KEY (server_id) REFERENCES Servers(id)
);

-- Records the Anti-Cheat has determined to require manual verification.
CREATE TABLE RecordsToCheck (
	id INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
	course_id INT2 UNSIGNED NOT NULL,
	mode_id INT1 UNSIGNED NOT NULL,
	style_id INT1 UNSIGNED NOT NULL,
	player_id INT4 UNSIGNED NOT NULL,
	server_id INT2 UNSIGNED NOT NULL,
	teleports INT2 UNSIGNED NOT NULL,

	-- Amount of ingame ticks it took to complete this run.
	ticks INT4 UNSIGNED NOT NULL,

	created_on DATETIME NOT NULL,

	PRIMARY KEY (id),
	FOREIGN KEY (course_id) REFERENCES Courses(id),
	FOREIGN KEY (mode_id) REFERENCES Modes(id),
	FOREIGN KEY (style_id) REFERENCES Styles(id),
	FOREIGN KEY (player_id) REFERENCES Players(id),
	FOREIGN KEY (server_id) REFERENCES Servers(id)
);

-- Records the Anti-Cheat has determined to be "cheated".
CREATE TABLE RecordsCheated (
	id INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
	course_id INT2 UNSIGNED NOT NULL,
	mode_id INT1 UNSIGNED NOT NULL,
	style_id INT1 UNSIGNED NOT NULL,
	player_id INT4 UNSIGNED NOT NULL,
	server_id INT2 UNSIGNED NOT NULL,
	teleports INT2 UNSIGNED NOT NULL,

	-- Amount of ingame ticks it took to complete this run.
	ticks INT4 UNSIGNED NOT NULL,

	created_on DATETIME NOT NULL,

	PRIMARY KEY (id),
	FOREIGN KEY (course_id) REFERENCES Courses(id),
	FOREIGN KEY (mode_id) REFERENCES Modes(id),
	FOREIGN KEY (style_id) REFERENCES Styles(id),
	FOREIGN KEY (player_id) REFERENCES Players(id),
	FOREIGN KEY (server_id) REFERENCES Servers(id)
);
