//! SQL queries that are used in multiple places.

/// SQL query for fetching players.
pub static SELECT: &str = r#"
	SELECT SQL_CALC_FOUND_ROWS
	  p.id,
	  p.name,
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
