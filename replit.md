# User Context MCP Server

## Overview
A Model Context Protocol (MCP) server implementing a unified user context layer for multiple AI agents. Creates a "Digital Self" that persists across all AI interactions.

## Tech Stack
- **Language:** Rust (Axum framework)
- **Database:** PostgreSQL with pgcrypto
- **Authentication:** Google OAuth 2.0 + API key-based agent auth
- **Runtime:** Tokio async

## Architecture
Four-layer data model:
- **Layer 0 (Public):** Language, timezone, preferences - accessible with any valid API key
- **Layer 1 (Context):** Active projects, goals, memories - requires layer1 permission
- **Layer 2 (Personal):** Identity, location, interests - requires layer2 permission
- **Layer 3 (Vault):** Encrypted secrets - requires explicit per-request consent

## Project Structure
```
src/
├── main.rs          - Axum server setup, routing
├── config.rs        - Environment configuration
├── error.rs         - Error types
├── models/          - Data types (user, api_key, context layers)
├── db/              - Database operations (pool, users, api_keys, layers, vault, audit)
├── auth/            - Authentication (Google OAuth, sessions, API key middleware)
├── api/             - REST API handlers (layers, keys, vault, audit)
├── mcp/             - MCP protocol implementation (server, tools)
├── crypto/          - Hashing and vault encryption (SHA-256, AES-256-GCM)
migrations/          - SQL migration files
static/dashboard/    - Management dashboard HTML
```

## Key Endpoints
- `GET /` - Dashboard UI
- `GET /health` - Health check
- `GET /auth/google` - Google OAuth login
- `GET/PUT /api/v0/context` - Layer 0 public context
- `GET/PUT /api/v1/context` - Layer 1 work context
- `GET/PUT /api/v2/context` - Layer 2 personal data
- `POST /api/v3/request` - Vault access request
- `POST /mcp` - MCP protocol endpoint
- `GET/POST /api/keys` - API key management

## Environment Variables Required
- `DATABASE_URL` - PostgreSQL connection string (auto-set)
- `GOOGLE_CLIENT_ID` - Google OAuth client ID
- `GOOGLE_CLIENT_SECRET` - Google OAuth client secret
- `GOOGLE_REDIRECT_URI` - OAuth callback URL
- `SESSION_SECRET` - Session encryption key

## Recent Changes
- 2026-02-18: Initial implementation of full MCP server with all 4 layers, Google OAuth, API key management, vault consent flow, audit logging, dashboard UI, and MCP protocol support.
