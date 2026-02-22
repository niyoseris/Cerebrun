use serde_json::{json, Value};

pub fn list_tools() -> Value {
    json!({
        "tools": [
            {
                "name": "get_context",
                "description": "Retrieve user context data for a specific layer. Layer 0: language, timezone, communication preferences. Layer 1: active projects, goals, pinned memories. Layer 2: personal identity info. Layer 3: encrypted vault (requires consent).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "layer": { "type": "integer", "enum": [0, 1, 2, 3] },
                        "fields": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["layer"]
                }
            },
            {
                "name": "update_context",
                "description": "Update user context data for a specific layer",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "layer": { "type": "integer", "enum": [0, 1, 2] },
                        "data": { "type": "object" }
                    },
                    "required": ["layer", "data"]
                }
            },
            {
                "name": "search_context",
                "description": "Semantic search across user's context layers and knowledge base using vector embeddings. Use this to find relevant context BEFORE injecting it into conversations. This prevents over-injection by only retrieving context items that are semantically relevant to the current topic. Returns ranked results with similarity scores. Requires an OpenAI or Ollama API key for embedding generation. Admin-configurable search limits and thresholds apply.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Natural language search query (e.g., 'Rust authentication implementation', 'user preferences for coding style')" },
                        "limit": { "type": "integer", "description": "Max results to return (overrides system default if provided)" },
                        "min_similarity": { "type": "number", "description": "Minimum similarity threshold 0.0-1.0 (overrides system default if provided)" },
                        "include_knowledge": { "type": "boolean", "description": "Also search Knowledge Base entries (default true)", "default": true }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "request_vault_access",
                "description": "Request access to vault data (requires user consent)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "reason": { "type": "string" },
                        "requested_fields": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["reason", "requested_fields"]
                }
            },
            {
                "name": "list_conversations",
                "description": "List the user's LLM Gateway conversations. Shows recent chat threads with different AI models (OpenAI, Gemini, Anthropic, Ollama). Use this to see what conversations the user has had.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "description": "Max conversations to return (default 20)", "default": 20 },
                        "provider": { "type": "string", "description": "Filter by provider: openai, gemini, anthropic, ollama" }
                    }
                }
            },
            {
                "name": "get_conversation",
                "description": "Get full conversation history including all messages. Use this when the user references a past conversation (e.g., 'what did I discuss with Gemini yesterday').",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "conversation_id": { "type": "string", "format": "uuid", "description": "UUID of the conversation" }
                    },
                    "required": ["conversation_id"]
                }
            },
            {
                "name": "search_conversations",
                "description": "Search through all conversation history by keyword. Searches both conversation titles and message content.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search keyword or phrase" },
                        "provider": { "type": "string", "description": "Filter by provider: openai, gemini, anthropic, ollama" },
                        "limit": { "type": "integer", "description": "Max results (default 5)", "default": 5 }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "chat_with_llm",
                "description": "Send a message to an LLM through the Gateway. Creates a new conversation or continues an existing one. Only Layer 0 preferences are auto-injected. Use search_context first to find relevant context, then include it in your message.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "message": { "type": "string", "description": "The message to send to the LLM" },
                        "provider": { "type": "string", "enum": ["openai", "gemini", "anthropic", "ollama"], "description": "LLM provider" },
                        "model": { "type": "string", "description": "Model name (e.g., gpt-4.1, gemini-3-flash, claude-sonnet-4.6, gpt-oss:120b-cloud)" },
                        "conversation_id": { "type": "string", "format": "uuid", "description": "Continue an existing conversation (optional)" },
                        "title": { "type": "string", "description": "Title for new conversation (optional)" }
                    },
                    "required": ["message", "provider", "model"]
                }
            },
            {
                "name": "fork_conversation",
                "description": "Fork a conversation at a specific message to continue with a different LLM.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "conversation_id": { "type": "string", "format": "uuid", "description": "Source conversation UUID" },
                        "message_id": { "type": "string", "format": "uuid", "description": "Fork point message UUID" },
                        "new_provider": { "type": "string", "enum": ["openai", "gemini", "anthropic", "ollama"] },
                        "new_model": { "type": "string", "description": "Model for the new fork" }
                    },
                    "required": ["conversation_id", "message_id", "new_provider", "new_model"]
                }
            },
            {
                "name": "get_llm_usage",
                "description": "Get token usage metrics for LLM Gateway. Shows total tokens used and breakdown by provider and model.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "push_knowledge",
                "description": "Store a categorized knowledge entry in the user's Knowledge Base. Content is automatically vectorized for semantic search. Categories: project_update, code_change, decision, learning, todo, insight, architecture, bug_fix, feature, note.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "content": { "type": "string", "description": "The main knowledge content to store" },
                        "summary": { "type": "string", "description": "A brief one-line summary of the content (optional but recommended)" },
                        "category": { "type": "string", "description": "Category: project_update, code_change, decision, learning, todo, insight, architecture, bug_fix, feature, note, or custom" },
                        "subcategory": { "type": "string", "description": "More specific subcategory (e.g., 'frontend', 'backend', 'database')" },
                        "tags": { "type": "array", "items": { "type": "string" }, "description": "Tags for filtering (e.g., ['rust', 'auth', 'performance'])" },
                        "source_project": { "type": "string", "description": "Name of the project this knowledge relates to" }
                    },
                    "required": ["content"]
                }
            },
            {
                "name": "query_knowledge",
                "description": "Search the user's Knowledge Base by keyword, category, tag, or project. For semantic search, use search_context instead.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "keyword": { "type": "string", "description": "Search keyword to find in content, summary, or raw input" },
                        "category": { "type": "string", "description": "Filter by category (e.g., 'project_update', 'decision', 'bug_fix')" },
                        "tag": { "type": "string", "description": "Filter by tag (e.g., 'rust', 'auth')" },
                        "source_project": { "type": "string", "description": "Filter by project name" },
                        "limit": { "type": "integer", "description": "Max results (default 20)", "default": 20 },
                        "offset": { "type": "integer", "description": "Skip first N results for pagination (default 0)", "default": 0 }
                    }
                }
            },
            {
                "name": "list_knowledge_categories",
                "description": "List all knowledge categories with entry counts.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }
        ]
    })
}
