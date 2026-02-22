# Cerebrun - Digital Self MCP Server + Multi-LLM Gateway

## Overview
A Model Context Protocol (MCP) server implementing a unified user context layer for multiple AI agents. Creates a "Digital Self" that persists across all AI interactions. Features a multi-LLM gateway supporting BYOK (Bring Your Own Key), SSE streaming, thread forking, A/B comparison, vector semantic search, and over-injection protection.

## Tech Stack
- **Language:** Rust (Axum framework)
- **Database:** PostgreSQL with pgcrypto + pgvector
- **Authentication:** Google OAuth 2.0 + API key-based agent auth
- **Runtime:** Tokio async
- **LLM Providers:** OpenAI, Google Gemini, Anthropic, Ollama Cloud (BYOK)
- **Embeddings:** OpenAI text-embedding-3-small / Ollama nomic-embed-text
- **Frontend:** Vanilla JS (no framework dependencies)

## Architecture
Four-layer data model:
- **Layer 0 (Public):** Language, timezone, preferences - accessible with any valid API key
- **Layer 1 (Context):** Active projects, goals, memories - requires layer1 permission
- **Layer 2 (Personal):** Identity, location, interests - requires layer2 permission
- **Layer 3 (Vault):** Encrypted secrets - requires explicit per-request consent

LLM Gateway features:
- **BYOK:** Users bring their own API keys, validated on input, encrypted at rest (AES-256-GCM)
- **Over-injection Protection:** Only Layer 0 auto-injected by default. Agents use search_context MCP tool for deeper, relevant context via vector similarity search
- **Conversations:** Persistent threads with full message history, inject_context toggle, token budget
- **Thread Forking:** Fork conversation at any message to different LLM
- **A/B Compare:** Same prompt to multiple LLMs simultaneously, side-by-side results
- **SSE Streaming:** Real-time token-by-token response streaming
- **Token Metrics:** Per-request token counting (no cost tracking - users track via their own provider accounts)

Vector Embedding features:
- **pgvector:** PostgreSQL extension for vector similarity search
- **Auto-embedding:** Knowledge entries and context items vectorized on save
- **search_context MCP tool:** Semantic search across context layers and knowledge base
- **Multilingual:** Embeddings work with Turkish/English queries seamlessly
- **Provider flexible:** Uses OpenAI or Ollama Cloud for embeddings (whichever key is available)

Knowledge Base features:
- **Agent-driven storage:** AI agents push categorized knowledge entries via MCP
- **Agent-side categorization:** The agent's own LLM handles categorization (no server-side LLM calls needed)
- **Categories:** project_update, code_change, decision, learning, todo, insight, architecture, bug_fix, feature, note, or custom
- **Tagging:** Flexible tag-based filtering
- **Source tracking:** Which agent and project generated each entry
- **Dashboard UI:** Knowledge Base tab with search, category filter, detail view, pagination

## Project Structure
```
src/
├── main.rs          - Axum server setup, routing
├── config.rs        - Environment configuration
├── error.rs         - Error types
├── models/          - Data types (user, api_key, context layers, llm, knowledge)
├── db/              - Database operations (pool, users, api_keys, layers, vault, audit, llm_keys, conversations, llm_usage, knowledge, embeddings)
├── auth/            - Authentication (Google OAuth, sessions, API key middleware)
├── api/             - REST API handlers (layers, keys, vault, audit, llm, knowledge)
├── mcp/             - MCP protocol implementation (server, tools)
├── crypto/          - Hashing and vault encryption (SHA-256, AES-256-GCM)
├── llm/             - LLM provider clients (provider - OpenAI, Gemini, Anthropic, Ollama Cloud + embedding support)
migrations/
├── 001_initial.sql  - Core tables
├── 002_llm_gateway.sql - LLM gateway tables
├── 003_knowledge_base.sql - Knowledge base table
├── 004_vector_embeddings.sql - pgvector extension, embedding columns, context_embeddings table
static/dashboard/
├── index.html       - Management dashboard (includes Knowledge Base tab)
├── chat.html        - LLM Gateway chat interface
```

## Key Endpoints

### MCP / Context
- `GET /` - Dashboard UI
- `GET /chat` - LLM Gateway chat UI
- `GET /health` - Health check
- `GET /auth/google` - Google OAuth login
- `GET/PUT /api/v0/context` - Layer 0 public context
- `GET/PUT /api/v1/context` - Layer 1 work context
- `GET/PUT /api/v2/context` - Layer 2 personal data
- `POST /api/v3/request` - Vault access request
- `POST /mcp` - MCP protocol endpoint
- `GET/POST /api/keys` - API key management

### LLM Gateway
- `GET/POST /api/llm/keys` - Provider key CRUD (BYOK)
- `DELETE /api/llm/keys/:id` - Delete provider key
- `GET /api/llm/models` - Available models per provider
- `GET/POST /api/llm/conversations` - Conversation CRUD (with inject_context, context_token_budget)
- `GET/DELETE /api/llm/conversations/:id` - Get/delete conversation
- `POST /api/llm/conversations/:id/chat` - Send message (non-streaming)
- `POST /api/llm/conversations/:id/stream` - Send message (SSE streaming)
- `POST /api/llm/conversations/:id/fork` - Fork conversation to new LLM
- `POST /api/llm/compare` - A/B comparison (multi-LLM parallel)
- `GET /api/llm/metrics` - Token usage metrics

### Knowledge Base
- `GET /api/knowledge` - List/search knowledge entries
- `GET /api/knowledge/:id` - Get specific entry
- `DELETE /api/knowledge/:id` - Delete entry

### MCP Tools (13 tools)
- `get_context` - Read context layers 0-3
- `update_context` - Update context layers 0-2 (auto-embeds Layer 1 items)
- `search_context` - Semantic vector search across context + knowledge (over-injection prevention)
- `request_vault_access` - Request vault consent
- `list_conversations` - List LLM conversations
- `get_conversation` - Get conversation with messages
- `search_conversations` - Keyword search conversations
- `chat_with_llm` - Send message to any LLM (OpenAI/Gemini/Anthropic/Ollama)
- `fork_conversation` - Fork conversation to different LLM
- `get_llm_usage` - Token usage metrics
- `push_knowledge` - Store knowledge entry (auto-embedded for vector search)
- `query_knowledge` - Search knowledge by keyword/category/tag
- `list_knowledge_categories` - List categories with counts

## Environment Variables Required
- `DATABASE_URL` - PostgreSQL connection string (auto-set)
- `GOOGLE_CLIENT_ID` - Google OAuth client ID
- `GOOGLE_CLIENT_SECRET` - Google OAuth client secret
- `GOOGLE_REDIRECT_URI` - OAuth callback URL (dynamic, auto-detected)
- `SESSION_SECRET` - Session encryption key

## Recent Changes
- 2026-02-22: Vector embeddings + over-injection protection - pgvector extension, auto-embedding of knowledge and context items, search_context MCP tool for semantic search, only Layer 0 auto-injected in conversations, inject_context toggle and token budget per conversation
- 2026-02-22: Ollama Cloud provider added - 4th LLM provider with OpenAI-compatible API, chat + embedding support (nomic-embed-text)
- 2026-02-22: Cost tracking removed - Users track costs via their own provider accounts. Token metrics retained.
- 2026-02-22: Model lists updated - OpenAI (GPT-4.1, o3, o4-mini), Gemini (3.1 Pro, 3 Flash, 2.5 series), Anthropic (Opus 4.6, Sonnet 4.6, Haiku 4.5), Ollama Cloud (qwen3-coder, gpt-oss, glm-4.6)
- 2026-02-21: Knowledge Base feature
- 2026-02-21: Multi-LLM Gateway expansion
- 2026-02-18: Initial implementation

## User Preferences
- English language for all UI text (no Turkish)
- Dark theme (bg #0f1117, cards #1a1d27, accent #7c8aff)
- Using rustls instead of native-tls (OpenSSL compatibility)
- No cost tracking on server - users track via provider accounts
