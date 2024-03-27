UPDATE
  Players
SET
  role_flags = (role_flags | (1 << 8))
WHERE
  id = 76561198282622073;
