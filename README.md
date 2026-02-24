# Cerebrun - Digital Self MCP Server + Multi-LLM Gateway

Cerebrun is a Model Context Protocol (MCP) server that provides a persistent memory and unified context layer for AI agents. It implements a 4-layer architecture to manage user preferences, work context, personal identity, and encrypted secrets.

## Features
- **4-Layer Context:** Granular control over what data is shared with AI agents.
- **Multi-LLM Gateway:** Support for OpenAI, Anthropic, Gemini, and Ollama with Bring Your Own Key (BYOK) model.
- **Vector Semantic Search:** Powered by `pgvector` for efficient context retrieval.
- **Cross-Conversation Memory:** Eliminates context fragmentation across different AI interactions.
- **Secure Vault:** Encrypted storage for sensitive data with per-request user consent.

## Tech Stack
- **Backend:** Rust (Axum, Tokio, SQLx)
- **Database:** PostgreSQL with `pgcrypto` and `pgvector`
- **Frontend:** Vanilla JavaScript (no framework overhead)
- **Auth:** Google OAuth 2.0

## License & Intellectual Property

This project is **Open Source**. You are free to view, study, and contribute to the code.

### Commercial Usage & Intellectual Property
**IMPORTANT:** While the source code is public, all **Intellectual Property (IP)** rights and ownership remain with the original author. **Commercial use, redistribution, or modification for commercial purposes is strictly prohibited** without explicit written permission from the owner.

---
© 2026 Cerebrun. All rights reserved.
