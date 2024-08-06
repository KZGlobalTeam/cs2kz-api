INSERT
  IGNORE INTO `Players` (`id`, `name`, `ip_address`)
VALUES
  (
    76561198260657129,
    "ReDMooN",
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
    "kz_grotto",
    "very cool map",
    0,
    3121168339,
    0x20a546b4fdaebc518c079a91e24738be
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
        name = "kz_grotto"
    ),
    76561198260657129
  );

INSERT INTO
  Courses (name, description, map_id)
VALUES
  (
    "Main",
    "it looks very nice",
    (
      SELECT
        id
      FROM
        Maps
      WHERE
        name = "kz_grotto"
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
            name = "kz_grotto"
        )
    ),
    76561198260657129
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
            name = "kz_grotto"
        )
    ),
    1,
    1,
    3,
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
            name = "kz_grotto"
        )
    ),
    1,
    0,
    4,
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
            name = "kz_grotto"
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
            name = "kz_grotto"
        )
    ),
    2,
    0,
    3,
    1
  );
