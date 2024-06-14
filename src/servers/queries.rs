//! Shared SQL queries.

/// SQL query for `SELECT`ing servers from the database.
pub static SELECT: &str = r#"
	SELECT SQL_CALC_FOUND_ROWS
	  s.id,
	  s.name,
	  s.host,
	  s.port,
	  p.name owner_name,
	  p.id owner_id,
	  s.created_on
	FROM
	  Servers s
	  JOIN Players p ON p.id = s.owner_id
"#;
