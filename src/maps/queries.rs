pub static BASE_SELECT: &str = r#"
	SELECT
	  m.id,
	  m.workshop_id,
	  m.name,
	  p2.steam_id mapper_steam_id,
	  p2.name mapper_name,
	  p2.is_banned mapper_is_banned,
	  c.id course_id,
	  c.name course_name,
	  c.description course_description,
	  c.map_stage course_stage,
	  p4.steam_id course_mapper_steam_id,
	  p4.name course_mapper_name,
	  p4.is_banned course_mapper_is_banned,
	  f.id filter_id,
	  f.mode_id filter_mode,
	  f.teleports filter_teleports,
	  f.tier filter_tier,
	  f.ranked_status filter_ranked_status,
	  f.notes filter_notes,
	  m.global_status,
	  m.description,
	  m.checksum,
	  m.created_on
	FROM
	  Maps m
	  JOIN Mappers p1 ON p1.map_id = m.id
	  JOIN Players p2 ON p2.steam_id = p1.player_id
	  JOIN Courses c ON c.map_id = m.id
	  JOIN CourseMappers p3 ON p3.course_id = c.id
	  JOIN Players p4 ON p4.steam_id = p3.player_id
	  JOIN CourseFilters f ON f.course_id = c.id
"#;
