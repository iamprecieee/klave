CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    instruction_type TEXT NOT NULL,
    status TEXT NOT NULL,
    tx_signature TEXT,
    policy_violations TEXT,
    metadata TEXT
);

CREATE INDEX IF NOT EXISTS idx_audit_agent_id ON audit_log(agent_id);
CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp);
