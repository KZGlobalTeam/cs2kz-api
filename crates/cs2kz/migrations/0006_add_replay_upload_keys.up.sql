CREATE TABLE IF NOT EXISTS ReplayUploadKeys (
  record_id BINARY(16) PRIMARY KEY NOT NULL REFERENCES Records(id) ON DELETE CASCADE,
  upload_key BINARY(16) NOT NULL,
  expires_at TIMESTAMP NOT NULL,
  CONSTRAINT UC_replay_upload_key UNIQUE (upload_key)
);
