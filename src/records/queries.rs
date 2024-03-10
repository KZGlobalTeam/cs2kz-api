/// Base query for `SELECT`ing records from the database.
pub static BASE_SELECT: &str = r#"
	SELECT
	  r.id,
	  p.steam_id player_id,
	  p.name player_name,
	  m.id map_id,
	  m.name map_name,
	  s.id server_id,
	  s.name server_name,
	  f.mode_id MODE,
	  r.style_id style,
	  r.teleports,
	  r.time,
	  r.perfs,
	  r.bhops_tick0,
	  r.bhops_tick1,
	  r.bhops_tick2,
	  r.bhops_tick3,
	  r.bhops_tick4,
	  r.bhops_tick5,
	  r.bhops_tick6,
	  r.bhops_tick7,
	  r.bhops_tick8,
	  v.version plugin_version,
	  r.created_on
	FROM
	  Records r
	  JOIN Players p ON p.steam_id = r.player_id
	  JOIN CourseFilters f ON f.id = r.filter_id
	  JOIN Courses c ON c.id = f.course_id
	  JOIN Maps m ON m.id = c.map_id
	  JOIN Servers s ON s.id = r.server_id
	  JOIN PluginVersions v ON v.id = r.plugin_version_id
"#;
