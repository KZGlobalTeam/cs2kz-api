/// Base query for `SELECT`ing course sessions from the database.
pub static BASE_SELECT: &str = r#"
	SELECT
	  s.id,
	  p.steam_id,
	  p.name player_name,
	  s.mode_id mode,
	  c.id course_id,
	  c.name course_name,
	  m.id map_id,
	  m.name map_name,
	  sv.id server_id,
	  sv.name server_name,
	  s.playtime,
	  s.total_runs,
	  s.finished_runs,
	  s.perfs,
	  s.bhops_tick0,
	  s.bhops_tick1,
	  s.bhops_tick2,
	  s.bhops_tick3,
	  s.bhops_tick4,
	  s.bhops_tick5,
	  s.bhops_tick6,
	  s.bhops_tick7,
	  s.bhops_tick8,
	  s.created_on
	FROM
	  CourseSessions s
	  JOIN Players p ON p.steam_id = s.player_id
	  JOIN Courses c ON c.id = s.course_id
	  JOIN Maps m ON m.id = c.map_id
	  JOIN Servers sv ON sv.id = s.server_id
"#;
