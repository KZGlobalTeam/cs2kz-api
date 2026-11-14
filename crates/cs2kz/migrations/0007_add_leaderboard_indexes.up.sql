CREATE INDEX IF NOT EXISTS IDX_BestNubRecords_filter_time ON BestNubRecords (filter_id, time, record_id);

CREATE INDEX IF NOT EXISTS IDX_BestProRecords_filter_time ON BestProRecords (filter_id, time, record_id);
