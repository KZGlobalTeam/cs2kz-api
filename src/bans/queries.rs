pub static BASE_SELECT: &str = r#"
	SELECT
	  b.id,
	  p1.steam_id,
	  p1.ip_address,
	  b.reason,
	  s.id server_id,
	  s.name server_name,
	  s.ip_address server_ip_address,
	  s.port server_port,
	  p2.steam_id server_owner_steam_id,
	  p2.name server_owner_name,
	  p2.is_banned server_owner_is_banned,
	  s.approved_on server_approved_on,
	  v.version p3.steam_id banned_by_steam_id,
	  p3.name banned_by_name,
	  p3.is_banned banned_by_is_banned,
	  b.created_on,
	  b.expires_on
	FROM
	  Bans b
	  JOIN Players p1 ON p1.steam_id = b.player_id
	  LEFT JOIN Servers s ON s.id = b.server_id
	  LEFT JOIN Players p2 ON p2.steam_id = s.owned_by
	  JOIN PluginVersions v ON v.id = b.plugin_version_id
	  LEFT JOIN Players p3 ON p3.steam_id = b.banned_by
"#;
