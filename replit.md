# User Context MCP Server + LLM Gateway

## Overview
A Model Context Protocol (MCP) server implementing a unified user context layer for multiple AI agents. Creates a "Digital Self" that persists across all AI interactions. Now expanded with a multi-LLM gateway supporting BYOK (Bring Your Own Key), SSE streaming, thread forking, A/B comparison, and token/cost tracking.

## Tech Stack
- **Language:** Rust (Axum framework)
- **Database:** PostgreSQL with pgcrypto
- **Authentication:** Google OAuth 2.0 + API key-based agent auth
- **Runtime:** Tokio async
- **LLM Providers:** OpenAI, Google Gemini, Anthropic (BYOK)
- **Frontend:** Vanilla JS (no framework dependencies)

## Architecture
Four-layer data model:
- **Layer 0 (Public):** Language, timezone, preferences - accessible with any valid API key
- **Layer 1 (Context):** Active projects, goals, memories - requires layer1 permission
- **Layer 2 (Personal):** Identity, location, interests - requires layer2 permission
- **Layer 3 (Vault):** Encrypted secrets - requires explicit per-request consent

LLM Gateway features:
- **BYOK:** Users bring their own API keys, validated on input, encrypted at rest (AES-256-GCM)
- **Context Injection:** User context from layers 0-2 auto-injected as system prompt
- **Conversations:** Persistent threads with full message history
- **Thread Forking:** Fork conversation at any message to different LLM
- **A/B Compare:** Same prompt to multiple LLMs simultaneously, side-by-side results
- **SSE Streaming:** Real-time token-by-token response streaming
- **Token Metrics:** Per-request token counting and cost tracking with dashboard

## Project Structure
```
src/
├── main.rs          - Axum server setup, routing
├── config.rs        - Environment configuration
├── error.rs         - Error types
├── models/          - Data types (user, api_key, context layers, llm)
├── db/              - Database operations (pool, users, api_keys, layers, vault, audit, llm_keys, conversations, llm_usage)
├── auth/            - Authentication (Google OAuth, sessions, API key middleware)
├── api/             - REST API handlers (layers, keys, vault, audit, llm)
├── mcp/             - MCP protocol implementation (server, tools)
├── crypto/          - Hashing and vault encryption (SHA-256, AES-256-GCM)
├── llm/             - LLM provider clients and pricing (provider, pricing)
migrations/
├── 001_initial.sql  - Core tables
├── 002_llm_gateway.sql - LLM gateway tables
static/dashboard/
├── index.html       - Management dashboard
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
- `GET/POST /api/llm/conversations` - Conversation CRUD
- `GET/DELETE /api/llm/conversations/:id` - Get/delete conversation
- `POST /api/llm/conversations/:id/chat` - Send message (non-streaming)
- `POST /api/llm/conversations/:id/stream` - Send message (SSE streaming)
- `POST /api/llm/conversations/:id/fork` - Fork conversation to new LLM
- `POST /api/llm/compare` - A/B comparison (multi-LLM parallel)
- `GET /api/llm/metrics` - Token usage and cost metrics

## Environment Variables Required
- `DATABASE_URL` - PostgreSQL connection string (auto-set)
- `GOOGLE_CLIENT_ID` - Google OAuth client ID
- `GOOGLE_CLIENT_SECRET` - Google OAuth client secret
- `GOOGLE_REDIRECT_URI` - OAuth callback URL
- `SESSION_SECRET` - Session encryption key

## Recent Changes
- 2026-02-21: Multi-LLM Gateway expansion - BYOK key management, conversation threading, SSE streaming, A/B comparison, thread forking, token metrics/cost tracking, full chat UI at /chat
- 2026-02-18: Initial implementation of full MCP server with all 4 layers, Google OAuth, API key management, vault consent flow, audit logging, dashboard UI, and MCP protocol support.

## User Preferences
- Turkish language support preferred for dashboard UI descriptions
- Dark theme (bg #0f1117, cards #1a1d27, accent #7c8aff)
- Using rustls instead of native-tls (OpenSSL compatibility)
