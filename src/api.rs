use anyhow::{Context, Result};
use reqwest::header::{self, HeaderMap, HeaderValue};
use std::collections::HashMap;

const BASE_URL: &str = "https://open.faceit.com/data/v4";

pub type MatchItem = HashMap<String, String>;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PlayerProfile {
    pub player_id: String,
    pub nickname: String,
    pub avatar: Option<String>,
    pub country: Option<String>,
    pub games: Option<PlayerGames>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PlayerGames {
    pub cs2: Option<Cs2Info>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Cs2Info {
    pub skill_level: Option<i32>,
    pub faceit_elo: Option<i32>,
    pub game_player_name: Option<String>,
    pub region: Option<String>,
}

impl PlayerProfile {
    pub fn cs2(&self) -> Option<&Cs2Info> {
        self.games.as_ref().and_then(|g| g.cs2.as_ref())
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PlayerLifetimeStats {
    pub player_id: String,
    pub game_id: String,
    pub lifetime: Option<HashMap<String, serde_json::Value>>,
    pub segments: Option<Vec<LifetimeSegment>>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct LifetimeSegment {
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    pub mode: Option<String>,
    pub label: Option<String>,
    pub img_regular: Option<String>,
    pub stats: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone)]
pub struct FaceitApi {
    client: reqwest::Client,
}

impl FaceitApi {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        
        let env_key = std::env::var("FACEIT_API_KEY").ok().filter(|k| !k.is_empty());
        let file_key = crate::config::load_api_key();
        let build_key = option_env!("FACEIT_API_KEY").filter(|k| !k.is_empty()).map(|k| k.to_string());

        let api_key = env_key.or(file_key).or(build_key).unwrap_or_default();
        
        if api_key.is_empty() {
            eprintln!("No API key found. Set FACEIT_API_KEY env var or put api_key in config.toml");
        }
        
        if !api_key.is_empty() {
            headers.insert(
                header::AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap(),
            );
        }
        
        headers.insert(
            header::ACCEPT,
            HeaderValue::from_static("application/json"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client");

        Self { client }
    }

    pub async fn search_player(&self, nickname: &str) -> Result<PlayerProfile> {
        let url = format!("{}/players?nickname={}", BASE_URL, urlencoding::encode(nickname));

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Network error: check your connection")?;

        let status = response.status();
        let text = response.text().await.context("Failed to read response")?;

        if !status.is_success() {
            anyhow::bail!(api_error(status.as_u16(), "search"));
        }

        let profile: PlayerProfile = serde_json::from_str(&text)
            .context("Failed to parse player data")?;

        Ok(profile)
    }

    pub async fn get_lifetime_stats(&self, player_id: &str) -> Result<PlayerLifetimeStats> {
        let url = format!("{}/players/{}/stats/cs2", BASE_URL, player_id);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Network error: check your connection")?;

        let status = response.status();
        let text = response.text().await.context("Failed to read response")?;

        if !status.is_success() {
            anyhow::bail!(api_error(status.as_u16(), "lifetime stats"));
        }

        let stats: PlayerLifetimeStats = serde_json::from_str(&text)
            .context("Failed to parse lifetime stats")?;

        Ok(stats)
    }

    pub async fn get_match_history(&self, player_id: &str, offset: usize, limit: usize) -> Result<Vec<MatchItem>> {
        let url = format!("{}/players/{}/games/cs2/stats?offset={}&limit={}", BASE_URL, player_id, offset, limit);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Network error: check your connection")?;

        let status = response.status();
        let text = response.text().await.context("Failed to read response")?;

        if !status.is_success() {
            anyhow::bail!(api_error(status.as_u16(), "match history"));
        }

        let json: serde_json::Value = serde_json::from_str(&text)
            .context("Failed to parse match history")?;
        let items = json["items"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|item| {
                let stats = item.get("stats")?.as_object()?;
                let mut map = HashMap::new();
                for (k, v) in stats {
                    let val = match v {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => fmt_val_num(n),
                        serde_json::Value::Bool(b) => b.to_string(),
                        serde_json::Value::Null => String::new(),
                        _ => v.to_string(),
                    };
                    map.insert(k.clone(), val);
                }
                Some(map)
            })
            .collect();

        Ok(items)
    }
}

fn api_error(status: u16, context: &str) -> String {
    match status {
        401 => format!("Invalid API key (401) — check your config.toml"),
        403 => format!("Access denied (403) — check API key permissions"),
        404 => format!("{}: not found (404)", context),
        429 => format!("Rate limited (429) — try again in a moment"),
        _ => format!("{} failed (HTTP {})", context, status),
    }
}

fn fmt_val_num(n: &serde_json::Number) -> String {
    if let Some(i) = n.as_i64() {
        return fmt_thousands(i);
    }
    if let Some(u) = n.as_u64() {
        return fmt_thousands(u as i64);
    }
    if let Some(f) = n.as_f64() {
        if f.fract().abs() < 1e-6 {
            return fmt_thousands(f as i64);
        }
        let int = f.trunc() as i64;
        let dec = ((f - f.trunc()).abs() * 100.0).round() as u32;
        return format!("{},{:02}", fmt_thousands(int), dec);
    }
    n.to_string()
}

fn fmt_thousands(n: i64) -> String {
    let s = n.to_string();
    let len = s.len();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 && c != '-' && !out.ends_with('-') {
            out.push('.');
        }
        out.push(c);
    }
    out
}
