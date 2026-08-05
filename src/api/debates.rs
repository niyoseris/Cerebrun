use axum::extract::{Path, State};
use axum::Json;
use serde_json::json;
use uuid::Uuid;

use crate::db;
use crate::error::AppError;
use crate::AppState;

/// Public debate viewer page (no login required)
pub async fn serve_debate_page(
    State(state): State<AppState>,
    Path(public_id): Path<String>,
) -> Result<axum::response::Html<String>, AppError> {
    let public_uuid: Uuid = public_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid debate ID".to_string()))?;

    let debate = db::debates::get_debate_by_public_id(&state.pool, public_uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Debate not found".to_string()))?;

    let participants = db::debates::list_participants(&state.pool, debate.id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let messages = db::debates::list_messages(&state.pool, debate.id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let debate_json = json!({
        "topic": debate.topic,
        "description": debate.description,
        "status": debate.status,
        "completion_criteria": debate.completion_criteria,
        "consensus_reached": debate.consensus_reached,
        "created_at": debate.created_at,
        "updated_at": debate.updated_at,
        "concluded_at": debate.concluded_at,
        "participants": participants.iter().map(|p| json!({
            "provider": p.provider,
            "model": p.model,
            "joined_at": p.joined_at,
        })).collect::<Vec<_>>(),
        "messages": messages.iter().map(|m| json!({
            "provider": m.provider,
            "model": m.model,
            "round": m.round,
            "role": m.role,
            "content": m.content,
            "criteria_proposal": m.criteria_proposal,
            "created_at": m.created_at,
        })).collect::<Vec<_>>(),
    });

    let json_data = serde_json::to_string(&debate_json).unwrap_or_default();
    let topic_escaped = debate.topic.replace('"', "\"").replace('<', "&lt;").replace('>', "&gt;");
    let html = format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Debate: {} — Cerebrun</title>
<style>
:root {{
  --bg: #faf8f5;
  --card: #ffffff;
  --text: #2d3436;
  --border: #e8e4dd;
  --accent: #6c5ce7;
  --green: #00b894;
  --orange: #fdcb6e;
  --red: #e17055;
}}
* {{ margin:0; padding:0; box-sizing:border-box; }}
body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; background: var(--bg); color: var(--text); min-height: 100vh; }}
.header {{ background: var(--card); border-bottom: 1px solid var(--border); padding: 16px 24px; position: sticky; top:0; z-index:10; }}
.header h1 {{ font-size: 18px; color: var(--accent); margin-bottom: 4px; }}
.header .meta {{ font-size: 13px; color: #888; display: flex; gap: 16px; flex-wrap: wrap; align-items: center; }}
.status-badge {{ padding: 3px 10px; border-radius: 12px; font-size: 12px; font-weight: 600; text-transform: uppercase; }}
.status-open {{ background: #d4edda; color: #155724; }}
.status-concluded {{ background: #cce5ff; color: #004085; }}
.status-closed {{ background: #f8d7da; color: #721c24; }}
.container {{ max-width: 900px; margin: 0 auto; padding: 24px; }}
.section {{ background: var(--card); border: 1px solid var(--border); border-radius: 12px; padding: 20px; margin-bottom: 16px; }}
.section h2 {{ font-size: 15px; color: var(--accent); margin-bottom: 12px; }}
.criteria-list {{ list-style: none; }}
.criteria-item {{ padding: 8px 12px; margin-bottom: 6px; border-radius: 8px; background: #f5f3ef; font-size: 14px; display: flex; align-items: center; gap: 8px; }}
.criteria-item::before {{ content: '✓'; color: var(--green); font-weight: bold; }}
.participants {{ display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 12px; }}
.participant-chip {{ padding: 4px 12px; border-radius: 16px; font-size: 12px; font-weight: 500; background: #eee6f8; color: var(--accent); border: 1px solid var(--accent); }}
.msg {{ background: var(--card); border: 1px solid var(--border); border-radius: 12px; padding: 16px; margin-bottom: 12px; }}
.msg-header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }}
.msg-provider {{ font-size: 13px; font-weight: 600; color: var(--accent); }}
.msg-round {{ font-size: 11px; color: #aaa; }}
.msg-role {{ font-size: 11px; padding: 2px 8px; border-radius: 4px; background: #f0ede8; color: #666; }}
.msg-content {{ font-size: 14px; line-height: 1.6; white-space: pre-wrap; word-break: break-word; }}
.msg-criteria {{ margin-top: 8px; padding: 8px; background: #fff9e6; border-radius: 6px; font-size: 12px; color: #8a6d3b; }}
.round-divider {{ text-align: center; color: #aaa; font-size: 12px; margin: 16px 0; position: relative; }}
.round-divider::before, .round-divider::after {{ content: ''; position: absolute; top: 50%; width: 35%; height: 1px; background: var(--border); }}
.round-divider::before {{ left: 0; }}
.round-divider::after {{ right: 0; }}
.empty {{ text-align: center; color: #aaa; padding: 40px; }}
.refresh-btn {{ background: var(--accent); color: white; border: none; padding: 8px 16px; border-radius: 8px; cursor: pointer; font-size: 13px; }}
.refresh-btn:hover {{ opacity: 0.9; }}
</style>
</head>
<body>
<div class="header">
  <h1>🧠 <span id="topic"></span></h1>
  <div class="meta">
    <span id="status-badge" class="status-badge"></span>
    <span id="created"></span>
    <span id="msg-count"></span>
    <button class="refresh-btn" onclick="loadDebate()">↻ Refresh</button>
  </div>
</div>
<div class="container" id="debate-container">
  <div class="empty">Loading debate...</div>
</div>
<script>
const DEBATE_DATA = {{}};

async function loadDebate() {{
  try {{
    // The data is embedded server-side
    const data = {};
    DEBATE_DATA.data = data;
    render(data);
  }} catch(e) {{
    document.getElementById('debate-container').innerHTML = '<div class="empty">Failed to load debate.</div>';
  }}
}}

function render(data) {{
  document.getElementById('topic').textContent = data.topic || 'Untitled Debate';
  const badge = document.getElementById('status-badge');
  const status = data.status || 'open';
  badge.textContent = status;
  badge.className = 'status-badge status-' + status;
  document.getElementById('created').textContent = data.created_at ? new Date(data.created_at).toLocaleString() : '';
  document.getElementById('msg-count').textContent = (data.messages || []).length + ' messages';

  let html = '';

  // Description
  if (data.description) {{
    html += '<div class="section"><p style="font-size:14px;line-height:1.6;color:#555;">' + escapeHtml(data.description) + '</p></div>';
  }}

  // Completion criteria
  if (data.completion_criteria && data.completion_criteria.length > 0) {{
    html += '<div class="section"><h2>Completion Criteria</h2><ul class="criteria-list">';
    data.completion_criteria.forEach(c => {{
      html += '<li class="criteria-item">' + escapeHtml(c) + '</li>';
    }});
    html += '</ul></div>';
  }}

  // Participants
  if (data.participants && data.participants.length > 0) {{
    html += '<div class="section"><h2>Participants</h2><div class="participants">';
    data.participants.forEach(p => {{
      html += '<span class="participant-chip">' + escapeHtml(p.provider) + ' / ' + escapeHtml(p.model) + '</span>';
    }});
    html += '</div></div>';
  }}

  // Messages grouped by round
  if (data.messages && data.messages.length > 0) {{
    let currentRound = 0;
    data.messages.forEach(msg => {{
      if (msg.round !== currentRound) {{
        currentRound = msg.round;
        html += '<div class="round-divider">Round ' + currentRound + '</div>';
      }}
      html += '<div class="msg">';
      html += '<div class="msg-header">';
      html += '<span class="msg-provider">' + escapeHtml(msg.provider) + ' / ' + escapeHtml(msg.model) + '</span>';
      html += '<span><span class="msg-role">' + escapeHtml(msg.role || 'argument') + '</span> <span class="msg-round">' + (msg.created_at ? new Date(msg.created_at).toLocaleTimeString() : '') + '</span></span>';
      html += '</div>';
      html += '<div class="msg-content">' + escapeHtml(msg.content) + '</div>';
      if (msg.criteria_proposal) {{
        html += '<div class="msg-criteria">📋 Proposed criteria: ' + escapeHtml(JSON.stringify(msg.criteria_proposal)) + '</div>';
      }}
      html += '</div>';
    }});
  }} else {{
    html += '<div class="empty">No messages yet.</div>';
  }}

  if (data.consensus_reached) {{
    html = '<div class="section" style="border-color: var(--green); background: #d4edda;"><h2 style="color: var(--green);">✅ Consensus Reached</h2><p style="font-size:14px;">All participants agreed on the completion criteria.</p></div>' + html;
  }}

  document.getElementById('debate-container').innerHTML = html;
}}

function escapeHtml(text) {{
  if (!text) return '';
  const div = document.createElement('div');
  div.textContent = typeof text === 'object' ? JSON.stringify(text) : text;
  return div.innerHTML;
}}

loadDebate();
</script>
</body>
</html>"#, json_data, topic_escaped);

    Ok(axum::response::Html(html))
}

/// Public API: get debate by public_id (no auth, read-only)
pub async fn get_public_debate(
    State(state): State<AppState>,
    Path(public_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let public_uuid: Uuid = public_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid debate ID".to_string()))?;

    let debate = db::debates::get_debate_by_public_id(&state.pool, public_uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Debate not found".to_string()))?;

    let participants = db::debates::list_participants(&state.pool, debate.id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let messages = db::debates::list_messages(&state.pool, debate.id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(json!({
        "topic": debate.topic,
        "description": debate.description,
        "status": debate.status,
        "completion_criteria": debate.completion_criteria,
        "consensus_reached": debate.consensus_reached,
        "created_at": debate.created_at,
        "updated_at": debate.updated_at,
        "concluded_at": debate.concluded_at,
        "participants": participants.iter().map(|p| json!({
            "provider": p.provider,
            "model": p.model,
            "joined_at": p.joined_at,
        })).collect::<Vec<_>>(),
        "messages": messages.iter().map(|m| json!({
            "provider": m.provider,
            "model": m.model,
            "round": m.round,
            "role": m.role,
            "content": m.content,
            "criteria_proposal": m.criteria_proposal,
            "created_at": m.created_at,
        })).collect::<Vec<_>>(),
    })))
}
/// Authenticated: list user's debates (for dashboard)
pub async fn list_user_debates(
    State(state): State<AppState>,
    session: crate::auth::SessionUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let debates = db::debates::list_debates(&state.pool, session.user.id, None, 50)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let result: Vec<serde_json::Value> = debates.iter().map(|d| {
        json!({
            "id": d.id,
            "public_id": d.public_id,
            "public_url": format!("https://cereb.run/debate/{}", d.public_id),
            "topic": d.topic,
            "status": d.status,
            "completion_criteria": d.completion_criteria,
            "consensus_reached": d.consensus_reached,
            "created_at": d.created_at,
            "updated_at": d.updated_at,
            "concluded_at": d.concluded_at,
        })
    }).collect();

    Ok(Json(json!({ "debates": result, "count": result.len() })))
}
