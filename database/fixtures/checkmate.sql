INSERT
  IGNORE INTO `Players` (`id`, `name`, `ip_address`)
VALUES
  (
    76561198165203332,
    "GameChaos",
    "::1"
  );

INSERT INTO
  Maps (
    name,
    description,
    global_status,
    workshop_id,
    `checksum`
  )
VALUES
  (
    "kz_checkmate",
    "very cool map",
    1,
    3070194623,
    0xac566bab2b04744657c4dc79f78957cf
  );

INSERT INTO
  Mappers (map_id, player_id)
VALUES
  (
    (
      SELECT
        id
      FROM
        Maps
      WHERE
        name = "kz_checkmate"
    ),
    76561198165203332
  );

INSERT INTO
  Courses (name, description, map_id)
VALUES
  (
    "Main",
    "the main course!",
    (
      SELECT
        id
      FROM
        Maps
      WHERE
        name = "kz_checkmate"
    )
  );

INSERT INTO
  CourseMappers (course_id, player_id)
VALUES
  (
    (
      SELECT
        id
      FROM
        Courses
      WHERE
        map_id = (
          SELECT
            id
          FROM
            Maps
          WHERE
            name = "kz_checkmate"
        )
    ),
    76561198165203332
  );

INSERT INTO
  CourseFilters (
    course_id,
    `mode`,
    teleports,
    tier,
    ranked_status
  )
VALUES
  (
    (
      SELECT
        id
      FROM
        Courses
      WHERE
        map_id = (
          SELECT
            id
          FROM
            Maps
          WHERE
            name = "kz_checkmate"
        )
    ),
    1,
    1,
    5,
    1
  );

INSERT INTO
  CourseFilters (
    course_id,
    `mode`,
    teleports,
    tier,
    ranked_status
  )
VALUES
  (
    (
      SELECT
        id
      FROM
        Courses
      WHERE
        map_id = (
          SELECT
            id
          FROM
            Maps
          WHERE
            name = "kz_checkmate"
        )
    ),
    1,
    0,
    6,
    1
  );

INSERT INTO
  CourseFilters (
    course_id,
    `mode`,
    teleports,
    tier,
    ranked_status
  )
VALUES
  (
    (
      SELECT
        id
      FROM
        Courses
      WHERE
        map_id = (
          SELECT
            id
          FROM
            Maps
          WHERE
            name = "kz_checkmate"
        )
    ),
    2,
    1,
    2,
    1
  );

INSERT INTO
  CourseFilters (
    course_id,
    `mode`,
    teleports,
    tier,
    ranked_status
  )
VALUES
  (
    (
      SELECT
        id
      FROM
        Courses
      WHERE
        map_id = (
          SELECT
            id
          FROM
            Maps
          WHERE
            name = "kz_checkmate"
        )
    ),
    2,
    0,
    3,
    1
  );
