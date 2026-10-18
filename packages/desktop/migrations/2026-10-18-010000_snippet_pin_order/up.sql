-- Position of a pinned snippet on the sidebar; NULL when unpinned.
ALTER TABLE host_snippets ADD COLUMN pin_order INTEGER;
