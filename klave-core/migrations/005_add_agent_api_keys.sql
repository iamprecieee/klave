-- Add API key fields to agents table
ALTER TABLE agents ADD COLUMN api_key_hash TEXT;
ALTER TABLE agents ADD COLUMN encrypted_api_key BLOB;

CREATE INDEX idx_agents_api_key_hash ON agents(api_key_hash);
