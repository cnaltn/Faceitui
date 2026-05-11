use std::fs;
use std::path::PathBuf;

const FILE_NAMES: &[&str] = &["config.toml", "faceit-stats.toml", ".faceit-stats"];

pub fn load_api_key() -> Option<String> {
    load_key("api_key")
}

pub fn load_ai_api_key() -> Option<String> {
    std::env::var("AI_API_KEY").ok().filter(|k| !k.is_empty())
        .or_else(|| option_env!("AI_API_KEY").filter(|k| !k.is_empty()).map(|k| k.to_string()))
}

pub fn load_theme_name() -> Option<String> {
    std::env::var("FACITER_THEME").ok().filter(|k| !k.is_empty())
        .or_else(|| load_key("theme"))
}

pub fn save_theme_name(name: &str) {
    let dirs = config_dirs();

    for dir in &dirs {
        for fname in FILE_NAMES {
            let path = dir.join(fname);
            if path.exists() {
                update_or_append_key(&path, "theme", name);
                return;
            }
        }
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let path = cwd.join(FILE_NAMES[0]);
    let _ = fs::write(&path, format!("theme = \"{}\"\n", name));
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

fn update_or_append_key(path: &PathBuf, key: &str, value: &str) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            let _ = fs::write(path, format!("{} = \"{}\"\n", key, value));
            return;
        }
    };

    let mut new_content = String::new();
    let mut found = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if !found && !trimmed.is_empty() && !trimmed.starts_with('#') {
            if let Some((k, _)) = trimmed.split_once('=') {
                if k.trim() == key {
                    new_content.push_str(&format!("{} = \"{}\"\n", key, value));
                    found = true;
                    continue;
                }
            }
        }
        new_content.push_str(line);
        new_content.push('\n');
    }

    if !found {
        if !content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push_str(&format!("{} = \"{}\"\n", key, value));
    }

    let _ = fs::write(path, new_content);
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
