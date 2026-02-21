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
                "description": "List the user's LLM Gateway conversations. Shows recent chat threads with different AI models (OpenAI, Gemini, Anthropic). Use this to see what conversations the user has had.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "description": "Max conversations to return (default 20)", "default": 20 },
                        "provider": { "type": "string", "description": "Filter by provider: openai, gemini, anthropic" }
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
                "description": "Search through all conversation history by keyword. Searches both conversation titles and message content. Use this when the user asks about a past discussion on a topic (e.g., 'find my conversation about Rust macros').",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search keyword or phrase" },
                        "provider": { "type": "string", "description": "Filter by provider: openai, gemini, anthropic" },
                        "limit": { "type": "integer", "description": "Max results (default 5)", "default": 5 }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "chat_with_llm",
                "description": "Send a message to an LLM through the Gateway. Creates a new conversation or continues an existing one. The user's context (preferences, goals, identity) is automatically injected. Use this when the user asks you to query another LLM (e.g., 'ask Gemini about this').",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "message": { "type": "string", "description": "The message to send to the LLM" },
                        "provider": { "type": "string", "enum": ["openai", "gemini", "anthropic"], "description": "LLM provider" },
                        "model": { "type": "string", "description": "Model name (e.g., gpt-4o, gemini-2.0-flash, claude-3-5-sonnet-latest)" },
                        "conversation_id": { "type": "string", "format": "uuid", "description": "Continue an existing conversation (optional)" },
                        "title": { "type": "string", "description": "Title for new conversation (optional)" }
                    },
                    "required": ["message", "provider", "model"]
                }
            },
            {
                "name": "fork_conversation",
                "description": "Fork a conversation at a specific message to continue with a different LLM. Copies the conversation history up to that point and creates a new thread with the specified provider/model.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "conversation_id": { "type": "string", "format": "uuid", "description": "Source conversation UUID" },
                        "message_id": { "type": "string", "format": "uuid", "description": "Fork point message UUID" },
                        "new_provider": { "type": "string", "enum": ["openai", "gemini", "anthropic"] },
                        "new_model": { "type": "string", "description": "Model for the new fork" }
                    },
                    "required": ["conversation_id", "message_id", "new_provider", "new_model"]
                }
            },
            {
                "name": "get_llm_usage",
                "description": "Get token usage and cost metrics for LLM Gateway. Shows total tokens used, total cost, and breakdown by provider and model.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }
        ]
    })
}
