//! SQL queries that are used in multiple places.

/// SQL query for fetching records.
pub static SELECT: &str = r#"
	SELECT
	  r.id,
	  f.mode_id MODE,
	  r.style_id style,
	  r.teleports,
	  r.time,
	  p.name player_name,
	  p.id player_id,
	  s.name server_name,
	  s.id server_id,
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
	  r.created_on
	FROM
	  Records r
	  JOIN CourseFilters f ON f.id = r.filter_id
	  JOIN Players p ON p.id = r.player_id
	  JOIN Servers s ON s.id = r.server_id
"#;
