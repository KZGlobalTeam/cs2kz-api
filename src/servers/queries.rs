pub static BASE_SELECT: &str = r#"
	SELECT
	  s.id,
	  s.name,
	  s.ip_address,
	  s.port,
	  p.steam_id owned_by_steam_id,
	  p.name owned_by_name,
	  s.approved_on
	FROM
	  Servers s
	  JOIN Players p ON p.steam_id = s.owned_by
"#;
