use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug)]
pub struct LlmResponse {
    pub content: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

#[derive(Debug)]
pub struct EmbeddingResponse {
    pub embedding: Vec<f32>,
    pub total_tokens: i32,
}

pub fn available_models(provider: &str) -> Vec<&'static str> {
    match provider {
        "openai" => vec![
            "gpt-4.1",
            "gpt-4.1-mini",
            "gpt-4.1-nano",
            "gpt-4o",
            "gpt-4o-mini",
            "o3",
            "o3-pro",
            "o4-mini",
            "o4-mini-high",
        ],
        "gemini" => vec![
            "gemini-3.1-pro",
            "gemini-3-pro",
            "gemini-3-flash",
            "gemini-2.5-pro",
            "gemini-2.5-flash",
            "gemini-2.5-flash-lite",
            "gemini-2.0-flash",
        ],
        "anthropic" => vec![
            "claude-opus-4.6",
            "claude-sonnet-4.6",
            "claude-haiku-4.5",
            "claude-opus-4.5",
            "claude-sonnet-4",
            "claude-haiku-4",
        ],
        "ollama" => vec![
            "qwen3-coder:480b-cloud",
            "gpt-oss:120b-cloud",
            "gpt-oss:20b-cloud",
            "glm-4.6:cloud",
            "qwen3.5:cloud",
        ],
        _ => vec![],
    }
}

pub fn supported_providers() -> Vec<&'static str> {
    vec!["openai", "gemini", "anthropic", "ollama"]
}

pub async fn call_llm(
    provider: &str,
    model: &str,
    api_key: &str,
    messages: &[LlmMessage],
) -> Result<LlmResponse, String> {
    let mut last_error = String::new();

    for attempt in 0..3 {
        if attempt > 0 {
            let delay = Duration::from_millis(500 * 2u64.pow(attempt as u32));
            tokio::time::sleep(delay).await;
        }

        match call_provider(provider, model, api_key, messages).await {
            Ok(response) => return Ok(response),
            Err(e) => {
                if e.contains("429") || e.contains("rate") || e.contains("timeout") {
                    last_error = e;
                    continue;
                }
                return Err(e);
            }
        }
    }

    Err(format!("All retries exhausted. Last error: {}", last_error))
}

async fn call_provider(
    provider: &str,
    model: &str,
    api_key: &str,
    messages: &[LlmMessage],
) -> Result<LlmResponse, String> {
    match provider {
        "openai" => call_openai(model, api_key, messages).await,
        "gemini" => call_gemini(model, api_key, messages).await,
        "anthropic" => call_anthropic(model, api_key, messages).await,
        "ollama" => call_ollama(model, api_key, messages).await,
        _ => Err(format!("Unsupported provider: {}", provider)),
    }
}

async fn call_openai(
    model: &str,
    api_key: &str,
    messages: &[LlmMessage],
) -> Result<LlmResponse, String> {
    let client = reqwest::Client::new();
    let body = json!({
        "model": model,
        "messages": messages,
        "max_tokens": 4096,
    });

    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(120))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("OpenAI request failed: {}", e))?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(format!("OpenAI error {}: {}", status.as_u16(), text));
    }

    let data: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let content = data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let usage = &data["usage"];

    Ok(LlmResponse {
        content,
        prompt_tokens: usage["prompt_tokens"].as_i64().unwrap_or(0) as i32,
        completion_tokens: usage["completion_tokens"].as_i64().unwrap_or(0) as i32,
        total_tokens: usage["total_tokens"].as_i64().unwrap_or(0) as i32,
    })
}

async fn call_gemini(
    model: &str,
    api_key: &str,
    messages: &[LlmMessage],
) -> Result<LlmResponse, String> {
    let client = reqwest::Client::new();

    let contents: Vec<serde_json::Value> = messages
        .iter()
        .filter(|m| m.role != "system")
        .map(|m| {
            json!({
                "role": if m.role == "assistant" { "model" } else { "user" },
                "parts": [{ "text": m.content }]
            })
        })
        .collect();

    let system_instruction = messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| json!({ "parts": [{ "text": m.content }] }));

    let mut body = json!({
        "contents": contents,
        "generationConfig": {
            "maxOutputTokens": 4096
        }
    });

    if let Some(sys) = system_instruction {
        body["systemInstruction"] = sys;
    }

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(120))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Gemini request failed: {}", e))?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(format!("Gemini error {}: {}", status.as_u16(), text));
    }

    let data: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let content = data["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let usage_meta = &data["usageMetadata"];
    let prompt_tokens = usage_meta["promptTokenCount"].as_i64().unwrap_or(0) as i32;
    let completion_tokens = usage_meta["candidatesTokenCount"].as_i64().unwrap_or(0) as i32;

    Ok(LlmResponse {
        content,
        prompt_tokens,
        completion_tokens,
        total_tokens: prompt_tokens + completion_tokens,
    })
}

async fn call_anthropic(
    model: &str,
    api_key: &str,
    messages: &[LlmMessage],
) -> Result<LlmResponse, String> {
    let client = reqwest::Client::new();

    let system_text = messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| m.content.clone());

    let api_messages: Vec<serde_json::Value> = messages
        .iter()
        .filter(|m| m.role != "system")
        .map(|m| json!({ "role": m.role, "content": m.content }))
        .collect();

    let mut body = json!({
        "model": model,
        "messages": api_messages,
        "max_tokens": 4096,
    });

    if let Some(sys) = system_text {
        body["system"] = json!(sys);
    }

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(120))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Anthropic request failed: {}", e))?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(format!("Anthropic error {}: {}", status.as_u16(), text));
    }

    let data: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let content = data["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let usage = &data["usage"];

    Ok(LlmResponse {
        content,
        prompt_tokens: usage["input_tokens"].as_i64().unwrap_or(0) as i32,
        completion_tokens: usage["output_tokens"].as_i64().unwrap_or(0) as i32,
        total_tokens: (usage["input_tokens"].as_i64().unwrap_or(0)
            + usage["output_tokens"].as_i64().unwrap_or(0)) as i32,
    })
}

async fn call_ollama(
    model: &str,
    api_key: &str,
    messages: &[LlmMessage],
) -> Result<LlmResponse, String> {
    let client = reqwest::Client::new();
    let body = json!({
        "model": model,
        "messages": messages,
        "stream": false,
    });

    let resp = client
        .post("https://ollama.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(180))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama Cloud request failed: {}", e))?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(format!("Ollama Cloud error {}: {}", status.as_u16(), text));
    }

    let data: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let content = data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let usage = &data["usage"];

    Ok(LlmResponse {
        content,
        prompt_tokens: usage["prompt_tokens"].as_i64().unwrap_or(0) as i32,
        completion_tokens: usage["completion_tokens"].as_i64().unwrap_or(0) as i32,
        total_tokens: usage["total_tokens"].as_i64().unwrap_or(0) as i32,
    })
}

pub async fn get_embedding(
    provider: &str,
    api_key: &str,
    text: &str,
) -> Result<EmbeddingResponse, String> {
    match provider {
        "openai" => get_openai_embedding(api_key, text).await,
        "ollama" => get_ollama_embedding(api_key, text).await,
        _ => Err(format!("Embedding not supported for provider: {}. Use OpenAI or Ollama.", provider)),
    }
}

async fn get_openai_embedding(api_key: &str, text: &str) -> Result<EmbeddingResponse, String> {
    let client = reqwest::Client::new();
    let body = json!({
        "model": "text-embedding-3-small",
        "input": text,
    });

    let resp = client
        .post("https://api.openai.com/v1/embeddings")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(30))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("OpenAI embedding request failed: {}", e))?;

    let status = resp.status();
    let text_resp = resp.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(format!("OpenAI embedding error {}: {}", status.as_u16(), text_resp));
    }

    let data: serde_json::Value = serde_json::from_str(&text_resp).map_err(|e| e.to_string())?;

    let embedding: Vec<f32> = data["data"][0]["embedding"]
        .as_array()
        .ok_or("No embedding in response")?
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect();

    let total_tokens = data["usage"]["total_tokens"].as_i64().unwrap_or(0) as i32;

    Ok(EmbeddingResponse { embedding, total_tokens })
}

async fn get_ollama_embedding(api_key: &str, text: &str) -> Result<EmbeddingResponse, String> {
    let client = reqwest::Client::new();
    let body = json!({
        "model": "nomic-embed-text",
        "input": text,
    });

    let resp = client
        .post("https://ollama.com/v1/embeddings")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(30))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama embedding request failed: {}", e))?;

    let status = resp.status();
    let text_resp = resp.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(format!("Ollama embedding error {}: {}", status.as_u16(), text_resp));
    }

    let data: serde_json::Value = serde_json::from_str(&text_resp).map_err(|e| e.to_string())?;

    let embedding: Vec<f32> = data["data"][0]["embedding"]
        .as_array()
        .ok_or("No embedding in response")?
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect();

    let total_tokens = data["usage"]["total_tokens"].as_i64().unwrap_or(0) as i32;

    Ok(EmbeddingResponse { embedding, total_tokens })
}

pub async fn validate_key(provider: &str, api_key: &str) -> Result<bool, String> {
    match provider {
        "openai" => {
            let resp = reqwest::Client::new()
                .get("https://api.openai.com/v1/models")
                .header("Authorization", format!("Bearer {}", api_key))
                .timeout(Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            Ok(resp.status().is_success())
        }
        "gemini" => {
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models?key={}",
                api_key
            );
            let resp = reqwest::Client::new()
                .get(&url)
                .timeout(Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            Ok(resp.status().is_success())
        }
        "anthropic" => {
            let resp = reqwest::Client::new()
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .json(&json!({
                    "model": "claude-haiku-4",
                    "max_tokens": 1,
                    "messages": [{"role": "user", "content": "hi"}]
                }))
                .timeout(Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            Ok(resp.status().is_success() || resp.status().as_u16() != 401)
        }
        "ollama" => {
            let resp = reqwest::Client::new()
                .get("https://ollama.com/v1/models")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .timeout(Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            
            let status = resp.status();
            if status.as_u16() == 401 {
                return Err("Ollama Cloud error 401: unauthorized. Please check your API key.".to_string());
            }
            Ok(status.is_success())
        }
        _ => Err(format!("Unknown provider: {}", provider)),
    }
}
