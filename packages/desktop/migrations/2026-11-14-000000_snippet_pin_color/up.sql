-- Color of a pinned snippet's sidebar icon as "#rrggbb"; NULL uses the default.
ALTER TABLE host_snippets ADD COLUMN pin_color TEXT;
