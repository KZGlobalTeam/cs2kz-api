//! SQL queries that are used in multiple places.

/// SQL query for fetching bans.
pub static SELECT: &str = r#"
	SELECT
	  b.id,
	  p.name player_name,
	  p.id player_id,
	  s.name server_name,
	  s.id server_id,
	  b.reason,
	  a.name admin_name,
	  a.id admin_id,
	  b.created_on,
	  b.expires_on,
	  ub.id unban_id,
	  ub.reason unban_reason,
	  a2.name unban_admin_name,
	  a2.id unban_admin_id,
	  ub.created_on unban_created_on
	FROM
	  Bans b
	  JOIN Players p ON p.id = b.player_id
	  LEFT JOIN Servers s ON s.id = b.server_id
	  LEFT JOIN Players a ON a.id = b.admin_id
	  LEFT JOIN Unbans ub ON ub.ban_id = b.id
	  LEFT JOIN Players a2 ON a2.id = ub.admin_id
"#;
