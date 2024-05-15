UPDATE
  Players
SET
  permissions = (permissions | (1 << 8))
WHERE
  id = 76561198282622073;
