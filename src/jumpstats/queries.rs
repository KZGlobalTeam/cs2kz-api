//! SQL queries that are used in multiple places.

/// SQL query for fetching jumpstats.
pub static SELECT: &str = r#"
	SELECT
	  j.id,
	  j.type,
	  j.mode_id MODE,
	  j.style_id style,
	  j.strafes,
	  j.distance,
	  j.sync,
	  j.pre,
	  j.max,
	  j.overlap,
	  j.bad_angles,
	  j.dead_air,
	  j.height,
	  j.airpath,
	  j.deviation,
	  j.average_width,
	  j.airtime,
	  p.name player_name,
	  p.id player_id,
	  s.name server_name,
	  s.id server_id,
	  j.created_on
	FROM
	  Jumpstats j
	  JOIN Players p ON p.id = j.player_id
	  JOIN Servers s ON s.id = j.server_id
"#;
