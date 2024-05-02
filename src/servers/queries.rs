//! SQL queries that are used in multiple places.

/// SQL query for fetching servers.
pub static SELECT: &str = r#"
	SELECT SQL_CALC_FOUND_ROWS
	  s.id,
	  s.name,
	  s.ip_address,
	  s.port,
	  p.name owner_name,
	  p.id owner_id,
	  s.created_on
	FROM
	  Servers s
	  JOIN Players p ON p.id = s.owner_id
"#;
