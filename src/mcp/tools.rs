use serde_json::{json, Value};

pub fn list_tools() -> Value {
    json!({
        "tools": [
            {
                "name": "get_context",
                "description": concat!(
                    "Retrieve user context data for a specific layer. ",
                    "Layer 0: language, timezone, communication preferences (always accessible). ",
                    "Layer 1: active projects, goals, pinned memories (requires layer1 permission). ",
                    "Layer 2: personal identity, location, interests (requires layer2 permission). ",
                    "Layer 3: encrypted vault — API keys, tokens, secrets. Requires 'vault' permission on your API key for direct access, or use request_vault_access for consent-based access. ",
                    "Start every session by reading Layer 0, then Layer 1 if you have permission."
                ),
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "layer": { "type": "integer", "enum": [0, 1, 2, 3], "description": "The context layer to read (0-3)" },
                        "fields": { "type": "array", "items": { "type": "string" }, "description": "Optional: specific fields to retrieve instead of the full layer" }
                    },
                    "required": ["layer"]
                }
            },
            {
                "name": "update_context",
                "description": concat!(
                    "Update user context data for a specific layer (0-3). Layers 0-2 use key-value data. ",
                    "Layer 3 (Vault) requires 'vault' permission and uses vault_key + data (string value) fields. ",
                    "IMPORTANT — choose the correct layer:\n",
                    "- Layer 0: General preferences — language, timezone, communication style, output format, blocked topics. ",
                    "Use when the user says things like 'speak Turkish', 'I prefer bullet points', 'use UTC+3'.\n",
                    "- Layer 1: Work context — active_projects (array of project names/descriptions), current_goals (array), ",
                    "working_directories (array of paths), pinned_memories (array of important things to remember). ",
                    "Use when the user mentions projects, sets goals, or says 'remember this'.\n",
                    "- Layer 2: Personal identity — display_name, location, interests (array), contact_preferences, relationship_notes. ",
                    "Use when the user shares personal info like their name, city, or hobbies.\n",
                    "- Layer 3: Vault (requires vault permission) — store secrets like API keys, tokens. ",
                    "Provide vault_key (name) and data (secret value as string). Example: {layer: 3, vault_key: 'OPENAI_API_KEY', data: 'sk-...'}.\n",
                    "NEVER store API keys or passwords in Layers 0-2 — use Layer 3 Vault."
                ),
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "layer": { "type": "integer", "enum": [0, 1, 2, 3], "description": "Target layer: 0 (preferences), 1 (work context), 2 (personal), or 3 (vault)" },
                        "data": { "type": "object", "description": "Key-value pairs to update (layers 0-2). Only provided fields are changed; omitted fields remain unchanged." },
                        "vault_key": { "type": "string", "description": "(Layer 3 only) Vault key name, e.g. 'OPENAI_API_KEY'" }
                    },
                    "required": ["layer"]
                }
            },
            {
                "name": "search_context",
                "description": concat!(
                    "Semantic vector search across the user's context layers and knowledge base. ",
                    "Use this BEFORE injecting context into conversations to find only relevant information and prevent token waste. ",
                    "Returns ranked results with similarity scores. ",
                    "This is the recommended way to retrieve deep context — instead of reading entire layers, search for what's relevant. ",
                    "Requires an embedding-capable API key (OpenAI or Ollama) configured in the dashboard."
                ),
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Natural language search query (e.g., 'Rust authentication implementation', 'user preferences for coding style')" },
                        "limit": { "type": "integer", "description": "Max results to return (default: system setting, typically 10)" },
                        "min_similarity": { "type": "number", "description": "Minimum similarity threshold 0.0-1.0 (default: system setting, typically 0.3)" },
                        "include_knowledge": { "type": "boolean", "description": "Also search Knowledge Base entries (default true)", "default": true }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "request_vault_access",
                "description": concat!(
                    "Submit a consent request to access the user's encrypted Vault (Layer 3). ",
                    "This is the CONSENT-BASED flow for API keys WITHOUT 'vault' permission. ",
                    "If your API key HAS 'vault' permission, use get_context with layer=3 instead — no consent needed. ",
                    "This tool creates a pending request that the user must approve via the dashboard. ",
                    "Once approved, the vault data becomes accessible with a temporary token.\n",
                    "IMPORTANT: To STORE new secrets in the vault, use update_context with layer=3 (requires vault permission). ",
                    "If the user wants to store a secret and your key lacks vault permission, direct them to the dashboard.",
                    "If the user pastes an API key in chat, store it via update_context(layer=3, vault_key=..., data=...) if you have vault permission."
                ),
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "reason": { "type": "string", "description": "Clear explanation of WHY you need vault access (shown to user for consent)" },
                        "requested_fields": { "type": "array", "items": { "type": "string" }, "description": "Specific vault fields to access (e.g., ['openai_api_key', 'github_token'])" }
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
                "description": "Search through all conversation history by keyword. Searches both conversation titles and message content. Use this when the user asks about past discussions.",
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
                "description": concat!(
                    "Send a message to an LLM through the Gateway. Creates a new conversation or continues an existing one. ",
                    "Only Layer 0 preferences are auto-injected. For deeper context, use search_context first, then include relevant results in your message. ",
                    "The user's provider API keys are managed through the Cerebrun dashboard — if no key exists for the requested provider, ",
                    "direct the user to Dashboard → LLM Keys to add one. ",
                    "Tip: Use conversation_id to continue an existing thread, or omit it to start a new one."
                ),
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "message": { "type": "string", "description": "The message to send to the LLM" },
                        "provider": { "type": "string", "enum": ["openai", "gemini", "anthropic", "ollama"], "description": "LLM provider" },
                        "model": { "type": "string", "description": "Model name (e.g., gpt-4.1, gemini-3-flash, claude-sonnet-4.6, gpt-oss:120b-cloud)" },
                        "conversation_id": { "type": "string", "format": "uuid", "description": "Continue an existing conversation (optional)" },
                        "title": { "type": "string", "description": "Title for new conversation (optional, auto-generated if omitted)" }
                    },
                    "required": ["message", "provider", "model"]
                }
            },
            {
                "name": "fork_conversation",
                "description": "Fork a conversation at a specific message to continue with a different LLM. Useful for comparing how different models would respond to the same context.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "conversation_id": { "type": "string", "format": "uuid", "description": "Source conversation UUID" },
                        "message_id": { "type": "string", "format": "uuid", "description": "Fork point message UUID — the new conversation will include all messages up to this point" },
                        "new_provider": { "type": "string", "enum": ["openai", "gemini", "anthropic", "ollama"] },
                        "new_model": { "type": "string", "description": "Model for the new fork" }
                    },
                    "required": ["conversation_id", "message_id", "new_provider", "new_model"]
                }
            },
            {
                "name": "get_llm_usage",
                "description": "Get token usage metrics for LLM Gateway. Shows total tokens used and breakdown by provider and model. Note: cost tracking is not available server-side — users track costs via their own provider accounts.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "push_knowledge",
                "description": concat!(
                    "Store a categorized knowledge entry in the user's Knowledge Base. Content is automatically vectorized for semantic search via search_context. ",
                    "Use this to persist important information across sessions:\n",
                    "- Project updates, architectural decisions, bug fixes, learnings, todos, insights\n",
                    "Best practices:\n",
                    "- Always include a category (project_update, code_change, decision, learning, todo, insight, architecture, bug_fix, feature, note)\n",
                    "- Always include a one-line summary for quick scanning\n",
                    "- Add relevant tags for keyword-based filtering (e.g., ['rust', 'auth', 'api'])\n",
                    "- Set source_project when the knowledge relates to a specific project\n",
                    "- NEVER store API keys or secrets here — use the Vault (Layer 3) instead"
                ),
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "content": { "type": "string", "description": "The main knowledge content to store" },
                        "summary": { "type": "string", "description": "A brief one-line summary (strongly recommended for better retrieval)" },
                        "category": { "type": "string", "description": "Category: project_update, code_change, decision, learning, todo, insight, architecture, bug_fix, feature, note" },
                        "subcategory": { "type": "string", "description": "More specific subcategory (e.g., 'frontend', 'backend', 'database')" },
                        "tags": { "type": "array", "items": { "type": "string" }, "description": "Tags for filtering (e.g., ['rust', 'auth', 'performance'])" },
                        "source_project": { "type": "string", "description": "Name of the project this knowledge relates to" }
                    },
                    "required": ["content"]
                }
            },
            {
                "name": "query_knowledge",
                "description": "Search the user's Knowledge Base by keyword, category, tag, or project. Returns exact keyword matches. For semantic/meaning-based search, use search_context instead.",
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
                "description": "List all knowledge categories with entry counts. Use this to understand what types of knowledge the user has stored.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "create_debate",
                "description": concat!(
                    "Create a new LLM debate in Cerebrun. ",
                    "A debate is an async discussion where multiple LLMs share their views on a topic and work toward consensus. ",
                    "You (the agent) provide the opening message and propose completion criteria. ",
                    "Returns a debate_id and a public_url that you can share with the user to follow the debate without logging in. ",
                    "The user can then ask other LLMs to join the debate using the debate_id."
                ),
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "topic": { "type": "string", "description": "The topic or question to debate" },
                        "description": { "type": "string", "description": "Optional longer description or context for the debate" },
                        "opening_message": { "type": "string", "description": "Your opening argument or position on the topic" },
                        "completion_criteria": { "type": "array", "items": { "type": "string" }, "description": "Proposed criteria for when the debate is considered resolved" },
                        "provider": { "type": "string", "enum": ["openai", "gemini", "anthropic", "ollama"], "description": "Your LLM provider" },
                        "model": { "type": "string", "description": "Your model name" }
                    },
                    "required": ["topic", "opening_message", "provider", "model"]
                }
            },
            {
                "name": "join_debate",
                "description": concat!(
                    "Join an existing debate in Cerebrun. ",
                    "Returns the full debate history (all previous messages, participants, and current completion criteria) so you can contribute meaningfully. ",
                    "You can agree with, rebut, or build upon previous arguments, and propose new completion criteria. ",
                    "Provide your response as the message field. Use criteria_agreement to indicate which existing criteria you agree with, and criteria_proposal to suggest new ones."
                ),
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "debate_id": { "type": "string", "description": "The debate UUID" },
                        "message": { "type": "string", "description": "Your argument, rebuttal, or contribution to the debate" },
                        "role": { "type": "string", "enum": ["argument", "rebuttal", "synthesis", "criteria_update"], "description": "Type of message (default: argument)" },
                        "provider": { "type": "string", "enum": ["openai", "gemini", "anthropic", "ollama"], "description": "Your LLM provider" },
                        "model": { "type": "string", "description": "Your model name" },
                        "criteria_agreement": { "type": "array", "items": { "type": "string" }, "description": "Existing criteria items you agree with" },
                        "criteria_proposal": { "type": "array", "items": { "type": "string" }, "description": "New criteria to propose for debate completion" }
                    },
                    "required": ["debate_id", "message", "provider", "model"]
                }
            },
            {
                "name": "get_debate",
                "description": "Get full debate details including all messages, participants, completion criteria, and status. Use this to read the current state of a debate before contributing.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "debate_id": { "type": "string", "description": "The debate UUID" }
                    },
                    "required": ["debate_id"]
                }
            },
            {
                "name": "list_debates",
                "description": "List all debates for this user. Filter by status (open, concluded, closed).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "status": { "type": "string", "enum": ["open", "concluded", "closed"], "description": "Filter by status (optional)" },
                        "limit": { "type": "integer", "description": "Max results (default 20)", "default": 20 }
                    }
                }
            },
            {
                "name": "conclude_debate",
                "description": "Conclude or close a debate. If all participants have agreed on the completion criteria, the debate is automatically concluded. The user can also force-close a debate.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "debate_id": { "type": "string", "description": "The debate UUID" },
                        "force": { "type": "boolean", "description": "Force close even if criteria not met (user intervention)" },
                        "reason": { "type": "string", "description": "Optional reason for concluding" }
                    },
                    "required": ["debate_id"]
                }
            },
            {
                "name": "get_version_history",
                "description": concat!(
                    "Get version history for a knowledge entry or context layer. ",
                    "Shows who changed what, when, and the full content snapshot at each version. ",
                    "Use this when the user asks about the history of a knowledge entry or context change. ",
                    "For knowledge entries, pass content_type='knowledge' and content_id=<knowledge UUID>. ",
                    "For context layers, pass content_type='layer0'/'layer1'/'layer2'/'vault' and content_id=<field name>."
                ),
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "content_type": { "type": "string", "enum": ["knowledge", "layer0", "layer1", "layer2", "vault", "debate_message"], "description": "Type of content" },
                        "content_id": { "type": "string", "description": "Content identifier (UUID for knowledge, field name for layers)" },
                        "limit": { "type": "integer", "description": "Max versions to return (default 20)", "default": 20 }
                    },
                    "required": ["content_type", "content_id"]
                }
            },
            {
                "name": "delete_knowledge",
                "description": concat!(
                    "Delete a knowledge entry. This is a SOFT DELETE — the entry is hidden but version history is preserved. ",
                    "IMPORTANT: Before deleting, show the user the full content of the entry and ask for confirmation. ",
                    "Tell the user exactly what will be deleted (show the content) and wait for their explicit approval. ",
                    "If the user also wants to permanently delete the version history, set force_purge=true (requires a SECOND confirmation from the user)."
                ),
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "knowledge_id": { "type": "string", "description": "The knowledge entry UUID to delete" },
                        "confirm": { "type": "boolean", "description": "Must be true to confirm deletion. Show content to user first." },
                        "force_purge": { "type": "boolean", "description": "If true, also permanently delete all version history for this entry. Requires explicit second confirmation from user." }
                    },
                    "required": ["knowledge_id", "confirm"]
                }
            },
            {
                "name": "update_knowledge",
                "description": concat!(
                    "Update an existing knowledge entry. The previous version is automatically saved in version history. ",
                    "Show the user the current content and the proposed changes, and get their confirmation before updating. ",
                    "You can update content, summary, category, subcategory, tags, and source_project. ",
                    "Only fields you provide will be changed; omitted fields remain unchanged."
                ),
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "knowledge_id": { "type": "string", "description": "The knowledge entry UUID to update" },
                        "content": { "type": "string", "description": "New content (optional if only updating metadata)" },
                        "summary": { "type": "string", "description": "New summary (optional)" },
                        "category": { "type": "string", "description": "New category (optional)" },
                        "subcategory": { "type": "string", "description": "New subcategory (optional)" },
                        "tags": { "type": "array", "items": { "type": "string" }, "description": "New tags (optional)" },
                        "source_project": { "type": "string", "description": "New source project (optional)" }
                    },
                    "required": ["knowledge_id"]
                }
            }
        ]
    })
}
