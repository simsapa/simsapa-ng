ALTER TABLE chanting_recordings ADD COLUMN volume REAL NOT NULL DEFAULT 1.0;
ALTER TABLE chanting_recordings ADD COLUMN playback_position_ms INTEGER NOT NULL DEFAULT 0;
