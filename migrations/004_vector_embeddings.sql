CREATE EXTENSION IF NOT EXISTS vector;

ALTER TABLE knowledge_entries ADD COLUMN IF NOT EXISTS embedding vector(1536);

CREATE TABLE IF NOT EXISTS context_embeddings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source_type VARCHAR(50) NOT NULL,
    source_key VARCHAR(255) NOT NULL,
    content_text TEXT NOT NULL,
    embedding vector(1536),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, source_type, source_key)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_embedding ON knowledge_entries USING hnsw (embedding vector_cosine_ops);
CREATE INDEX IF NOT EXISTS idx_context_embedding ON context_embeddings USING hnsw (embedding vector_cosine_ops);
CREATE INDEX IF NOT EXISTS idx_context_embeddings_user ON context_embeddings(user_id);

ALTER TABLE conversations ADD COLUMN IF NOT EXISTS inject_context BOOLEAN DEFAULT true;
ALTER TABLE conversations ADD COLUMN IF NOT EXISTS context_token_budget INTEGER DEFAULT 2000;
