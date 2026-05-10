use anyhow::Result;
use serde::Serialize;
use tokio::sync::mpsc;

const AI_URL: &str = "https://opencode.ai/zen/go/v1/chat/completions";
const MODEL: &str = "deepseek-v4-flash";

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

pub async fn analyze_player_streaming(
    name: &str,
    lifetime_summary: &str,
    maps_summary: &str,
    matches_summary: &str,
    tx: mpsc::UnboundedSender<Result<String, String>>,
    ai_key: Option<String>,
) -> Result<()> {
    let system_prompt = "You are a professional CS2 analyst. Provide sharp, insightful commentary in Turkish. Keep it concise (max 5 sentences). Focus on strengths, weaknesses, and notable patterns. Use stats to back your claims.";

    let user_prompt = format!(
        "Analyze this FACEIT CS2 player:\n\n\
        Player: {}\n\n\
        Lifetime Stats:\n{}\n\n\
        Map Performance:\n{}\n\n\
        Recent Matches (last 20):\n{}\n\n\
        Give a brief analysis in Turkish.",
        name, lifetime_summary, maps_summary, matches_summary
    );

    let body = ChatRequest {
        model: MODEL.to_string(),
        messages: vec![
            Message { role: "system".to_string(), content: system_prompt.to_string() },
            Message { role: "user".to_string(), content: user_prompt },
        ],
        max_tokens: 8192,
        temperature: 0.7,
        stream: true,
    };

    let ai_key = ai_key.unwrap_or_default();

    let client = reqwest::Client::new();
    let mut req = client
        .post(AI_URL)
        .json(&body);

    if !ai_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", ai_key));
    }

    let resp = req.send().await
        .map_err(|e| {
            let _ = tx.send(Err(format!("Connection failed: {}", e)));
            anyhow::anyhow!("{}", e)
        })?;
    let status = resp.status();

    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        let msg = format!("AI API error ({}): {}", status, &text[..text.len().min(300)]);
        let _ = tx.send(Err(msg.clone()));
        anyhow::bail!("{}", msg);
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();

    use futures_util::StreamExt;
    while let Some(result) = stream.next().await {
        let chunk = result?;
        let text = String::from_utf8_lossy(&chunk);
        buf.push_str(&text);

        // Parse SSE lines
        while let Some(pos) = buf.find('\n') {
            let line = buf[..pos].trim().to_string();
            buf = buf[pos + 1..].to_string();

            if line.is_empty() || line.starts_with(':') {
                continue;
            }
            if line.starts_with("data: ") {
                let data = &line[6..];
                if data == "[DONE]" {
                    break;
                }
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                        let _ = tx.send(Ok(content.to_string()));
                    }
                }
            }
        }
    }

    Ok(())
}
