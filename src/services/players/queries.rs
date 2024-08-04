//! Shared SQL queries.

/// SQL query for fetching players from the database.
pub const SELECT: &str = r#"
	SELECT SQL_CALC_FOUND_ROWS
	  p.id player_id,
	  p.name player_name,
	  p.ip_address,
	  (
	    SELECT
	      COUNT(b.id)
	    FROM
	      Bans b
	    WHERE
	      b.player_id = p.id
	      AND b.expires_on > NOW()
	  ) is_banned
	FROM
	  Players p
"#;
