-- What MCP clients (AI agents) may do on this host without asking:
-- 'ask' shows the approval dialog, 'allow' runs without it, 'deny' refuses.
ALTER TABLE ssh_hosts ADD COLUMN ai_access TEXT NOT NULL DEFAULT 'ask'
    CHECK (ai_access IN ('ask', 'allow', 'deny'));
