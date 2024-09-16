//! SQL queries.

/// SQL query for fetching servers from the database.
pub const SELECT: &str = r#"
	SELECT SQL_CALC_FOUND_ROWS
	  s.id,
	  s.name,
	  s.host,
	  s.port,
	  o.name owner_name,
	  o.id owner_id,
	  t.name tag_name,
	  s.created_on
	FROM
	  Servers s
	  JOIN Players o ON o.id = s.owner_id
	  JOIN ServerTags t ON t.server_id = s.id
"#;
