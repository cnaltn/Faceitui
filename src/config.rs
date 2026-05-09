use std::fs;
use std::path::PathBuf;

const FILE_NAMES: &[&str] = &["config.toml", "faceit-stats.toml", ".faceit-stats"];

pub fn load_api_key() -> Option<String> {
    load_key("api_key")
}

pub fn load_ai_api_key() -> Option<String> {
    std::env::var("AI_API_KEY").ok().filter(|k| !k.is_empty())
        .or_else(|| load_key("ai_api_key"))
        .or_else(|| option_env!("AI_API_KEY").filter(|k| !k.is_empty()).map(|k| k.to_string()))
}

pub fn load_theme_name() -> Option<String> {
    std::env::var("FACITER_THEME").ok().filter(|k| !k.is_empty())
        .or_else(|| load_key("theme"))
}

fn load_key(target: &str) -> Option<String> {
    let dirs = config_dirs();

    for dir in &dirs {
        for name in FILE_NAMES {
            let path = dir.join(name);
            if let Some(key) = try_read(&path, target) {
                return Some(key);
            }
        }
    }

    None
}

fn config_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd);
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.to_path_buf());
        }
    }

    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        dirs.push(PathBuf::from(home));
    }

    dirs
}

fn try_read(path: &PathBuf, target: &str) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            if key.trim() == target {
                let val = value.trim().trim_matches('"').trim();
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        } else if target == "api_key" && !trimmed.starts_with('[') && trimmed.len() > 10 {
            return Some(trimmed.to_string());
        }
    }
    None
}
