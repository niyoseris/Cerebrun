CREATE TABLE knowledge_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    summary VARCHAR(500),
    category VARCHAR(100) NOT NULL DEFAULT 'uncategorized',
    subcategory VARCHAR(100),
    tags TEXT[] DEFAULT '{}',
    source_agent VARCHAR(255),
    source_project VARCHAR(255),
    raw_input TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_knowledge_user ON knowledge_entries(user_id);
CREATE INDEX idx_knowledge_category ON knowledge_entries(user_id, category);
CREATE INDEX idx_knowledge_created ON knowledge_entries(created_at);
CREATE INDEX idx_knowledge_tags ON knowledge_entries USING GIN(tags);
