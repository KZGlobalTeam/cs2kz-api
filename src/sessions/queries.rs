pub static BASE_SELECT: &str = r#"
	SELECT
	  s.id,
	  p.steam_id,
	  p.name player_name,
	  sv.id server_id,
	  sv.name server_name,
	  sv.ip_address server_ip_address,
	  sv.port server_port,
	  sv_o.steam_id server_owner_steam_id,
	  sv_o.name server_owner_name,
	  sv.approved_on server_approved_on,
	  s.time_active,
	  s.time_spectating,
	  s.time_afk,
	  s.perfs,
	  s.bhops_tick0,
	  s.bhops_tick1,
	  s.bhops_tick2,
	  s.bhops_tick3,
	  s.bhops_tick4,
	  s.bhops_tick5,
	  s.bhops_tick6,
	  s.bhops_tick7,
	  s.bhops_tick8,
	  s.created_on
	FROM
	  Sessions s
	  JOIN Players p ON p.steam_id = s.player_id
	  JOIN Servers sv ON sv.id = s.server_id
	  JOIN Players sv_o ON sv_o.steam_id = sv.owned_by
"#;
