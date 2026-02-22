CREATE TABLE IF NOT EXISTS agent_policies (
    id TEXT PRIMARY KEY NOT NULL,
    agent_id TEXT NOT NULL UNIQUE REFERENCES agents(id),
    allowed_programs TEXT NOT NULL DEFAULT '[]',
    max_lamports_per_tx INTEGER NOT NULL DEFAULT 1000000000,
    token_allowlist TEXT NOT NULL DEFAULT '[]',
    daily_spend_limit_usd REAL NOT NULL DEFAULT 0,
    daily_swap_volume_usd REAL NOT NULL DEFAULT 0,
    slippage_bps INTEGER NOT NULL DEFAULT 50,
    withdrawal_destinations TEXT NOT NULL DEFAULT '[]',
    updated_at INTEGER NOT NULL
);
