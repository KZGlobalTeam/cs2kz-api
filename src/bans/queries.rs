/// Base query for `SELECT`ing bans from the database.
///
/// See [`Ban`]'s [`FromRow`] implementation for details on how the results are parsed.
///
/// [`Ban`]: crate::bans::Ban
/// [`FromRow`]: sqlx::FromRow
pub static BASE_SELECT: &str = r#"
	SELECT
	  b.id,
	  b.player_id,
	  p1.name player_name,
	  b.player_ip,
	  b.reason,
	  s.id server_id,
	  s.name server_name,
	  s.ip_address server_ip_address,
	  s.port server_port,
	  p2.steam_id server_owner_steam_id,
	  p2.name server_owner_name,
	  s.approved_on server_approved_on,
	  v.version plugin_version,
	  p3.steam_id banned_by_steam_id,
	  p3.name banned_by_name,
	  b.created_on,
	  b.expires_on,
	  ub.id unban_id,
	  ub.reason unban_reason,
	  ub.created_on unban_created_on,
	  p4.steam_id unbanned_by_steam_id,
	  p4.name unbanned_by_name
	FROM
	  Bans b
	  JOIN Players p1 ON p1.steam_id = b.player_id
	  LEFT JOIN Servers s ON s.id = b.server_id
	  LEFT JOIN Players p2 ON p2.steam_id = s.owned_by
	  JOIN PluginVersions v ON v.id = b.plugin_version_id
	  LEFT JOIN Players p3 ON p3.steam_id = b.banned_by
	  LEFT JOIN Unbans ub ON ub.ban_id = b.id
	  LEFT JOIN Players p4 ON p4.steam_id = ub.unbanned_by
"#;
