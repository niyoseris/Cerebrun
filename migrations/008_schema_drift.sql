-- Schema drift fixes: columns that are used by the application but were added
-- manually to production instead of through a migration. Keep every statement
-- idempotent so this file is safe to run on both fresh and existing databases.

ALTER TABLE users ADD COLUMN IF NOT EXISTS is_admin BOOLEAN DEFAULT FALSE;

ALTER TABLE conversations ADD COLUMN IF NOT EXISTS inject_context BOOLEAN DEFAULT true;
ALTER TABLE conversations ADD COLUMN IF NOT EXISTS context_token_budget INTEGER DEFAULT 2000;
ALTER TABLE conversations ADD COLUMN IF NOT EXISTS use_tools BOOLEAN DEFAULT FALSE;
