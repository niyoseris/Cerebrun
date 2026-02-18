use serde_json::{json, Value};

pub fn list_tools() -> Value {
    json!({
        "tools": [
            {
                "name": "get_context",
                "description": "Retrieve user context data for a specific layer",
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
            }
        ]
    })
}
