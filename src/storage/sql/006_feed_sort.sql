-- Add sort column to feeds table to persist sort preference per feed

ALTER TABLE feeds ADD COLUMN sort TEXT NOT NULL DEFAULT 'position';
