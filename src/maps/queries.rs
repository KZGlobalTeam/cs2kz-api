//! SQL queries that are used in multiple places.

/// SQL query for fetching maps.
pub static SELECT: &str = r#"
	SELECT
	  m.id,
	  m.name,
	  m.description,
	  m.global_status,
	  m.workshop_id,
	  m.checksum,
	  p1.id mapper_id,
	  p1.name mapper_name,
	  c.id course_id,
	  c.name course_name,
	  c.description course_description,
	  p2.id course_mapper_id,
	  p2.name course_mapper_name,
	  f.id filter_id,
	  f.mode_id filter_mode,
	  f.teleports filter_teleports,
	  f.tier filter_tier,
	  f.ranked_status filter_ranked_status,
	  f.notes filter_notes,
	  m.created_on
	FROM
	  Maps m
	  JOIN Mappers ON Mappers.map_id = m.id
	  JOIN Players p1 ON p1.id = Mappers.player_id
	  JOIN Courses c ON c.map_id = m.id
	  JOIN CourseMappers ON CourseMappers.course_id = c.id
	  JOIN Players p2 ON p2.id = CourseMappers.player_id
	  JOIN CourseFilters f ON f.course_id = c.id
"#;
