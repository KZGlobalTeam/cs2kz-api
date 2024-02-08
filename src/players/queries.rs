pub static GET_FULL_PLAYER: &str = r#"
	SELECT
	  p.steam_id,
	  p.name,
	  (
	    SELECT
	      COUNT(b.id)
	    FROM
	      Bans b
	    WHERE
	      b.player_id = p.steam_id
	      AND b.expires_on > NOW()
	  ) is_banned
	FROM
	  Players p
"#;
