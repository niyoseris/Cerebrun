-- Version history and audit tracking for knowledge entries and context layers
-- Tracks who changed what, when, with full content snapshots

CREATE TABLE IF NOT EXISTS content_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content_type VARCHAR(50) NOT NULL, -- knowledge, layer0, layer1, layer2, vault, debate_message
    content_id VARCHAR(255) NOT NULL, -- UUID or composite key (e.g., "layer1:active_projects")
    version_number INTEGER NOT NULL DEFAULT 1,
    content_snapshot TEXT NOT NULL, -- full content at this version
    summary_snapshot TEXT, -- optional summary snapshot (for knowledge)
    changed_by_provider VARCHAR(50), -- which LLM provider made the change
    changed_by_model VARCHAR(100), -- which model
    changed_by_api_key VARCHAR(255), -- API key name
    changed_by_user BOOLEAN DEFAULT FALSE, -- true if changed via dashboard (not MCP)
    action VARCHAR(50) NOT NULL DEFAULT 'update', -- create, update, delete
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, content_type, content_id, version_number)
);

CREATE INDEX idx_content_versions_user ON content_versions(user_id);
CREATE INDEX idx_content_versions_content ON content_versions(user_id, content_type, content_id);
CREATE INDEX idx_content_versions_created ON content_versions(created_at);

-- Add soft delete columns to knowledge_entries
ALTER TABLE knowledge_entries ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;
ALTER TABLE knowledge_entries ADD COLUMN IF NOT EXISTS deleted_by_provider VARCHAR(50);
ALTER TABLE knowledge_entries ADD COLUMN IF NOT EXISTS deleted_by_model VARCHAR(100);
ALTER TABLE knowledge_entries ADD COLUMN IF NOT EXISTS deleted_by_api_key VARCHAR(255);

-- Add soft delete to debate_messages
ALTER TABLE debate_messages ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;
ALTER TABLE debate_messages ADD COLUMN IF NOT EXISTS deleted_by_provider VARCHAR(50);
ALTER TABLE debate_messages ADD COLUMN IF NOT EXISTS deleted_by_model VARCHAR(100);