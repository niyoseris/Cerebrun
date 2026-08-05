-- Debate system tables
CREATE TABLE IF NOT EXISTS debates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    public_id UUID UNIQUE DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    api_key_id UUID REFERENCES api_keys(id) ON DELETE SET NULL,
    topic TEXT NOT NULL,
    description TEXT,
    status VARCHAR(50) DEFAULT 'open', -- open, concluded, closed
    completion_criteria JSONB DEFAULT '[]',
    consensus_reached BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    concluded_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS debate_participants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    debate_id UUID NOT NULL REFERENCES debates(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL,
    model VARCHAR(100) NOT NULL,
    api_key_name VARCHAR(255),
    joined_at TIMESTAMPTZ DEFAULT NOW(),
    criteria_agreed JSONB DEFAULT '[]', -- which criteria this participant agrees with
    UNIQUE(debate_id, provider, model)
);

CREATE TABLE IF NOT EXISTS debate_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    debate_id UUID NOT NULL REFERENCES debates(id) ON DELETE CASCADE,
    participant_id UUID REFERENCES debate_participants(id) ON DELETE SET NULL,
    provider VARCHAR(50) NOT NULL,
    model VARCHAR(100) NOT NULL,
    round INTEGER NOT NULL DEFAULT 1,
    role VARCHAR(50) NOT NULL DEFAULT 'argument', -- opening, argument, rebuttal, synthesis, criteria_update, user_intervention
    content TEXT NOT NULL,
    criteria_proposal JSONB, -- optional: proposed completion criteria
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_debates_user ON debates(user_id);
CREATE INDEX idx_debates_public_id ON debates(public_id);
CREATE INDEX idx_debates_status ON debates(status);
CREATE INDEX idx_debate_participants_debate ON debate_participants(debate_id);
CREATE INDEX idx_debate_messages_debate ON debate_messages(debate_id);
CREATE INDEX idx_debate_messages_round ON debate_messages(debate_id, round);