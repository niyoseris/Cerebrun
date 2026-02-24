# Cerebrun - Unified AI Memory Gateway & MCP Server

Cerebrun is more than just a Model Context Protocol (MCP) server; it is a powerful **AI API Gateway** and **Cross-Model Memory Layer**. It creates a unified intelligence substrate that persists across different AI providers (OpenAI, Anthropic, Gemini, Ollama) and different models from the same provider.

## The Concept: Unified AI Memory
Most AI interactions today are fragmented—each model has its own "vacuum" of context. Cerebrun solves this by providing:
- **Common Memory Socket:** A shared memory space that any AI agent or LLM can plug into.
- **Cross-Model Continuity:** Discuss a project with Gemini today, and have Claude recall the specific architecture decisions tomorrow.
- **Provider Agnostic Gateway:** A single API server that routes requests to multiple providers while injecting the same persistent user context.
- **Universal Identity:** Your preferences, coding style, and personal context (Layer 0-2) are automatically synchronized across all your AI tools.

## Key Features
- **4-Layer Context Architecture:** Granular control over what data is shared (Public, Context, Personal, Vault).
- **Multi-LLM Gateway:** Support for OpenAI, Anthropic, Gemini, and Ollama Cloud with Bring Your Own Key (BYOK) model.
- **Semantic Cross-Memory:** Powered by `pgvector` for vector similarity search across all previous conversations and knowledge entries.
- **Thread Forking:** Start a conversation with one model and fork it to another without losing context.
- **Secure Vault (Layer 3):** AES-256-GCM encrypted storage for sensitive data with per-request user consent.

## Tech Stack
- **Backend:** Rust (Axum, Tokio, SQLx)
- **Database:** PostgreSQL with `pgcrypto` and `pgvector`
- **Frontend:** Vanilla JavaScript (Management Dashboard & Chat UI)
- **Protocol:** MCP (Model Context Protocol) + REST API

## License & Intellectual Property

This project is **Open Source**. You are free to view, study, and contribute to the code.

### Commercial Usage & Intellectual Property
**IMPORTANT:** While the source code is public, all **Intellectual Property (IP)** rights and ownership—including the **source code**, **underlying concepts**, **architecture**, and **innovative methodologies**—remain exclusively with the original author. **Commercial use, redistribution, or modification for commercial purposes is strictly prohibited** without explicit written permission from the owner.

---
© 2026 Cerebrun. All rights reserved.
