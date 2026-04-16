-- Add is_user_added marker to chanting_recordings. A recording's recording_type
-- (reference vs user) is independent from whether the recording was seeded by
-- bootstrap or added at runtime -- a user may add their own reference audio.
-- Default 1 so runtime inserts are treated as user-added. Bootstrap must set 0.

ALTER TABLE chanting_recordings ADD COLUMN is_user_added BOOLEAN NOT NULL DEFAULT 1;
