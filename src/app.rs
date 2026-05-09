use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};
use std::time::Instant;
use tui_scrollview::ScrollViewState;
use crate::theme::AppTheme;

use crate::api::{FaceitApi, MatchItem, PlayerLifetimeStats, PlayerProfile};
use tokio::sync::mpsc;

pub type AppResult<T> = Result<T>;

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, PartialEq)]
pub enum LoadingState {
    Idle,
    Loading,
    Success,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub created_at: Instant,
    pub duration_secs: u64,
    pub is_error: bool,
}

impl Toast {
    pub fn info(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            created_at: Instant::now(),
            duration_secs: 3,
            is_error: false,
        }
    }
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            created_at: Instant::now(),
            duration_secs: 4,
            is_error: true,
        }
    }
    pub fn expired(&self) -> bool {
        self.created_at.elapsed().as_secs() >= self.duration_secs
    }
}

#[derive(Debug)]
pub struct App {
    pub theme: AppTheme,
    pub input: String,
    pub input_mode: InputMode,
    pub player_info: Option<PlayerProfile>,
    pub lifetime_stats: Option<PlayerLifetimeStats>,
    pub match_history: Option<Vec<MatchItem>>,
    pub loading_state: LoadingState,
    pub api: FaceitApi,
    pub selected_tab: usize,
    pub selected_row: usize,
    pub lifetime_sv: ScrollViewState,
    pub matches_sv: ScrollViewState,
    pub maps_sv: ScrollViewState,
    pub spinner_frame: usize,
    pub toasts: Vec<Toast>,
    pub show_help: bool,
    pub search_history: Vec<String>,
    pub history_index: Option<usize>,
    pub last_content_rect: Option<Rect>,
    pub input_rect: Option<Rect>,
    pub player_card_rect: Option<Rect>,
    pub show_match_popup: bool,
    pub match_popup_sv: ScrollViewState,
    pub show_map_popup: bool,
    pub map_popup_sv: ScrollViewState,
    pub current_page: usize,
    pub has_more_pages: bool,
    pub ai_response: Option<String>,
    pub ai_loading: bool,
    pub ai_sv: ScrollViewState,
    pub ai_partial: String,
    pub theme_names: Vec<String>,
    pub theme_index: usize,
    pub show_theme_selector: bool,
    pub theme_sv: ScrollViewState,
    pub theme_selector_row: usize,
    #[allow(dead_code)]
    ai_rx: Option<mpsc::UnboundedReceiver<String>>,
}

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const PAGE_SIZE: usize = 30;

impl App {
    pub fn new() -> Self {
        let theme_names: Vec<String> = opaline::list_available_themes()
            .into_iter()
            .map(|t| t.name)
            .collect();
        let theme_name = crate::config::load_theme_name()
            .unwrap_or_else(|| "flexoki-dark".to_string());
        let theme = AppTheme::load(&theme_name);
        let theme_index = theme_names.iter()
            .position(|t| t == &theme_name)
            .unwrap_or(0);
        Self {
            theme,
            input: String::new(),
            input_mode: InputMode::Normal,
            player_info: None,
            lifetime_stats: None,
            match_history: None,
            loading_state: LoadingState::Idle,
            api: FaceitApi::new(),
            selected_tab: 0,
            selected_row: 0,
            lifetime_sv: ScrollViewState::new(),
            matches_sv: ScrollViewState::new(),
            maps_sv: ScrollViewState::new(),
            spinner_frame: 0,
            toasts: vec![],
            show_help: false,
            search_history: vec![],
            history_index: None,
            last_content_rect: None,
            input_rect: None,
            player_card_rect: None,
            show_match_popup: false,
            match_popup_sv: ScrollViewState::new(),
            show_map_popup: false,
            map_popup_sv: ScrollViewState::new(),
            current_page: 0,
            has_more_pages: false,
            ai_response: None,
            ai_loading: false,
            ai_sv: ScrollViewState::new(),
            ai_partial: String::new(),
            theme_names,
            theme_index,
            show_theme_selector: false,
            theme_sv: ScrollViewState::new(),
            theme_selector_row: 0,
            ai_rx: None,
        }
    }

    pub fn tick(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER.len();
        self.toasts.retain(|t| !t.expired());

        if let Some(ref mut rx) = self.ai_rx {
            let mut closed = false;
            loop {
                match rx.try_recv() {
                    Ok(chunk) => {
                        self.ai_partial.push_str(&chunk);
                    }
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        closed = true;
                        break;
                    }
                }
            }
            if closed {
                self.ai_rx = None;
                if self.ai_partial.is_empty() {
                    self.ai_loading = false;
                    self.add_toast(Toast::error("AI returned empty response"));
                } else {
                    self.ai_response = Some(std::mem::take(&mut self.ai_partial));
                    self.ai_loading = false;
                    self.ai_sv = ScrollViewState::new();
                }
            }
        }
    }

    pub fn spinner(&self) -> &'static str {
        SPINNER[self.spinner_frame]
    }

    pub fn add_toast(&mut self, toast: Toast) {
        self.toasts.push(toast);
    }

    pub fn tab_item_count(&self) -> usize {
        match self.selected_tab {
            0 => self.lifetime_stats.as_ref().and_then(|s| s.lifetime.as_ref()).map(|m| m.len()).unwrap_or(0),
            1 => self.match_history.as_ref().map(|v| v.len()).unwrap_or(0),
            2 => self.lifetime_stats.as_ref()
                .and_then(|s| s.segments.as_ref())
                .map(|v| v.iter().filter(|s| s.type_field.as_deref() == Some("Map")).count())
                .unwrap_or(0),
            _ => 0,
        }
    }

    fn ensure_visible(&mut self) {
        match self.selected_tab {
            0 => self.lifetime_sv.set_offset(Position::new(0, self.selected_row as u16)),
            1 => self.matches_sv.set_offset(Position::new(0, self.selected_row as u16)),
            2 => self.maps_sv.set_offset(Position::new(0, self.selected_row as u16)),
            _ => {}
        }
    }

    pub async fn handle_key_event(&mut self, key: KeyEvent) -> AppResult<bool> {
        if self.show_theme_selector {
            match key.code {
                KeyCode::Esc => self.show_theme_selector = false,
                KeyCode::Enter => {
                    if self.theme_selector_row < self.theme_names.len() {
                        self.theme_index = self.theme_selector_row;
                        let name = &self.theme_names[self.theme_index];
                        self.theme = AppTheme::load(name);
                        self.add_toast(Toast::info(format!("Theme: {}", name)));
                    }
                    self.show_theme_selector = false;
                }
                KeyCode::Up => {
                    self.theme_selector_row = self.theme_selector_row.saturating_sub(1);
                    self.theme_sv.set_offset(Position::new(0, self.theme_selector_row as u16));
                }
                KeyCode::Down => {
                    let max = self.theme_names.len().saturating_sub(1);
                    if self.theme_selector_row < max {
                        self.theme_selector_row += 1;
                    }
                    self.theme_sv.set_offset(Position::new(0, self.theme_selector_row as u16));
                }
                KeyCode::Home => {
                    self.theme_selector_row = 0;
                    self.theme_sv.scroll_to_top();
                }
                KeyCode::End => {
                    self.theme_selector_row = self.theme_names.len().saturating_sub(1);
                    self.theme_sv.scroll_to_bottom();
                }
                KeyCode::PageUp => {
                    let rows = 15.min(self.theme_names.len().saturating_sub(1));
                    self.theme_selector_row = self.theme_selector_row.saturating_sub(rows);
                    self.theme_sv.set_offset(Position::new(0, self.theme_selector_row as u16));
                }
                KeyCode::PageDown => {
                    let max = self.theme_names.len().saturating_sub(1);
                    let rows = 15;
                    self.theme_selector_row = (self.theme_selector_row + rows).min(max);
                    self.theme_sv.set_offset(Position::new(0, self.theme_selector_row as u16));
                }
                _ => {}
            }
            return Ok(false);
        }

        if self.show_match_popup {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.show_match_popup = false,
                KeyCode::Up => {
                    self.match_popup_sv.scroll_up();
                }
                KeyCode::Down => {
                    self.match_popup_sv.scroll_down();
                }
                KeyCode::PageUp => {
                    self.match_popup_sv.scroll_page_up();
                }
                KeyCode::PageDown => {
                    self.match_popup_sv.scroll_page_down();
                }
                _ => {}
            }
            return Ok(false);
        }

        if self.show_map_popup {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.show_map_popup = false,
                KeyCode::Up => {
                    self.map_popup_sv.scroll_up();
                }
                KeyCode::Down => {
                    self.map_popup_sv.scroll_down();
                }
                KeyCode::PageUp => {
                    self.map_popup_sv.scroll_page_up();
                }
                KeyCode::PageDown => {
                    self.map_popup_sv.scroll_page_down();
                }
                _ => {}
            }
            return Ok(false);
        }

        if self.ai_response.is_some() {
            match key.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.ai_response = None;
                    self.ai_sv = ScrollViewState::new();
                }
                KeyCode::Up => {
                    self.ai_sv.scroll_up();
                }
                KeyCode::Down => {
                    self.ai_sv.scroll_down();
                }
                KeyCode::PageUp => {
                    self.ai_sv.scroll_page_up();
                }
                KeyCode::PageDown => {
                    self.ai_sv.scroll_page_down();
                }
                _ => {}
            }
            return Ok(false);
        }

        if self.show_help {
            self.show_help = false;
            return Ok(false);
        }

        match self.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Esc => {
                    self.loading_state = LoadingState::Idle;
                    self.player_info = None;
                    self.lifetime_stats = None;
                    self.match_history = None;
                    self.selected_tab = 0;
                    self.selected_row = 0;
                    self.current_page = 0;
                    self.has_more_pages = false;
                    self.ai_response = None;
                    self.ai_loading = false;
                    self.ai_partial.clear();
                    self.lifetime_sv = ScrollViewState::new();
                    self.matches_sv = ScrollViewState::new();
                    self.maps_sv = ScrollViewState::new();
                    self.ai_sv = ScrollViewState::new();
                    self.show_match_popup = false;
                    self.show_map_popup = false;
                    self.show_help = false;
                }
                KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(true),
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    self.input_mode = InputMode::Editing;
                    self.history_index = None;
                }
                KeyCode::Enter => {
                    if self.selected_tab == 1 && self.loading_state == LoadingState::Success {
                        if let Some(history) = &self.match_history {
                            if !history.is_empty() && self.selected_row < history.len() {
                                self.show_match_popup = true;
                                self.match_popup_sv = ScrollViewState::new();
                            }
                        }
                    } else if self.selected_tab == 2 && self.loading_state == LoadingState::Success {
                        let map_count = self.tab_item_count();
                        if map_count > 0 && self.selected_row < map_count {
                            self.show_map_popup = true;
                            self.map_popup_sv = ScrollViewState::new();
                        }
                    } else {
                        self.input_mode = InputMode::Editing;
                        self.history_index = None;
                    }
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    if !self.input.is_empty() {
                        self.fetch_stats().await?;
                    }
                }
                KeyCode::Char('?') => {
                    self.show_help = true;
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    if let Some(profile) = &self.player_info {
                        match copy_to_clipboard(&profile.player_id) {
                            Ok(_) => self.add_toast(Toast::info(format!("Copied: {}", profile.player_id))),
                            Err(e) => self.add_toast(Toast::error(format!("Copy failed: {}", e))),
                        }
                    } else if !self.input.is_empty() && self.input.len() == 36 {
                        match copy_to_clipboard(&self.input) {
                            Ok(_) => self.add_toast(Toast::info(format!("Copied: {}", self.input))),
                            Err(e) => self.add_toast(Toast::error(format!("Copy failed: {}", e))),
                        }
                    }
                }
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    self.show_theme_selector = true;
                    self.theme_selector_row = self.theme_index;
                    self.theme_sv = ScrollViewState::new();
                    self.theme_sv.set_offset(Position::new(0, self.theme_index.saturating_sub(7) as u16));
                }
                KeyCode::Tab => {
                    self.selected_tab = (self.selected_tab + 1) % 3;
                    self.selected_row = 0;
                    match self.selected_tab {
                        0 => self.lifetime_sv.scroll_to_top(),
                        1 => self.matches_sv.scroll_to_top(),
                        2 => self.maps_sv.scroll_to_top(),
                        _ => {}
                    }
                }
                KeyCode::Up => {
                    if self.selected_row > 0 {
                        self.selected_row -= 1;
                        self.ensure_visible();
                    }
                }
                KeyCode::Down => {
                    let max = self.tab_item_count().saturating_sub(1);
                    if self.selected_row < max {
                        self.selected_row += 1;
                        self.ensure_visible();
                    }
                }
                KeyCode::PageUp => {
                    let rows_per_page = 10;
                    if self.selected_row >= rows_per_page {
                        self.selected_row -= rows_per_page;
                    } else {
                        self.selected_row = 0;
                    }
                    self.ensure_visible();
                }
                KeyCode::PageDown => {
                    let max = self.tab_item_count().saturating_sub(1);
                    let rows_per_page = 10;
                    self.selected_row = (self.selected_row + rows_per_page).min(max);
                    self.ensure_visible();
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    if self.loading_state == LoadingState::Success && !self.ai_loading {
                        self.ai_load();
                    }
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    self.export_data();
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    if self.selected_tab == 1 && self.has_more_pages {
                        self.fetch_page(self.current_page + 1).await?;
                    }
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    if self.selected_tab == 1 && self.current_page > 0 {
                        self.fetch_page(self.current_page - 1).await?;
                    }
                }
                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Enter => {
                    if !self.input.trim().is_empty() {
                        // Add to history
                        if !self.search_history.contains(&self.input) {
                            self.search_history.push(self.input.clone());
                            if self.search_history.len() > 10 {
                                self.search_history.remove(0);
                            }
                        }
                        self.fetch_stats().await?;
                    }
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Char(c) => {
                    self.input.push(c);
                    self.history_index = None;
                }
                KeyCode::Backspace => {
                    self.input.pop();
                    self.history_index = None;
                }
                KeyCode::Delete => {
                    self.input.clear();
                    self.history_index = None;
                }
                KeyCode::Up => {
                    if !self.search_history.is_empty() {
                        let idx = match self.history_index {
                            Some(i) => i.saturating_sub(1),
                            None => self.search_history.len() - 1,
                        };
                        if idx < self.search_history.len() {
                            self.history_index = Some(idx);
                            self.input = self.search_history[idx].clone();
                        }
                    }
                }
                KeyCode::Down => {
                    if let Some(idx) = self.history_index {
                        if idx + 1 < self.search_history.len() {
                            self.history_index = Some(idx + 1);
                            self.input = self.search_history[idx + 1].clone();
                        } else {
                            self.history_index = None;
                            self.input.clear();
                        }
                    }
                }
                _ => {}
            },
        }
        Ok(false)
    }

    pub async fn fetch_stats(&mut self) -> AppResult<()> {
        let query = self.input.trim().to_string();
        self.loading_state = LoadingState::Loading;
        self.scroll_reset();
        self.player_info = None;
        self.lifetime_stats = None;
        self.match_history = None;

        let is_uuid = query.len() == 36 && query.contains('-');

        let player_id = if is_uuid {
            query.clone()
        } else {
            match self.api.search_player(&query).await {
                Ok(profile) => {
                    let id = profile.player_id.clone();
                    self.player_info = Some(profile);
                    id
                }
                Err(e) => {
                    self.loading_state = LoadingState::Error(format!("{}", e));
                    return Ok(());
                }
            }
        };

        let lifetime_future = self.api.get_lifetime_stats(&player_id);
        let history_future = self.api.get_match_history(&player_id, 0, PAGE_SIZE);

        let (lifetime_result, history_result) = tokio::join!(lifetime_future, history_future);

        match lifetime_result {
            Ok(stats) => {
                self.lifetime_stats = Some(stats);
            }
            Err(e) => {
                eprintln!("Lifetime stats error: {}", e);
            }
        }

        match history_result {
            Ok(history) => {
                self.has_more_pages = history.len() >= PAGE_SIZE;
                self.match_history = Some(history);
            }
            Err(e) => {
                eprintln!("Match history error: {}", e);
            }
        }

        if self.lifetime_stats.is_some() || self.match_history.is_some() {
            self.loading_state = LoadingState::Success;
            self.add_toast(Toast::info("Stats loaded successfully"));
        } else {
            self.loading_state = LoadingState::Error("No data found. Check the player name.".to_string());
        }

        Ok(())
    }

    fn scroll_reset(&mut self) {
        self.selected_row = 0;
        self.lifetime_sv = ScrollViewState::new();
        self.matches_sv = ScrollViewState::new();
        self.maps_sv = ScrollViewState::new();
        self.current_page = 0;
        self.has_more_pages = false;
        self.show_match_popup = false;
    }

    fn ai_load(&mut self) {
        let name = self.player_info.as_ref()
            .map(|p| p.nickname.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let lifetime = if let Some(stats) = &self.lifetime_stats {
            stats.lifetime.as_ref().map(|lt| {
                let mut items: Vec<_> = lt.iter().collect();
                items.sort_by(|a, b| a.0.cmp(b.0));
                items.iter()
                    .take(20)
                    .map(|(k, v)| {
                        let text = if k.as_str() == "Recent Results" {
                            if let serde_json::Value::Array(arr) = v {
                                arr.iter()
                                    .map(|el| match el.as_str() {
                                        Some("1") => "W", _ => "L",
                                    })
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            } else {
                                value_to_text(v)
                            }
                        } else {
                            value_to_text(v)
                        };
                        format!("{}: {}", k, text)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }).unwrap_or_default()
        } else { String::new() };

        let maps = if let Some(stats) = &self.lifetime_stats {
            stats.segments.as_ref().map(|segs| {
                segs.iter()
                    .filter(|s| s.type_field.as_deref() == Some("Map"))
                    .filter_map(|s| {
                        let label = s.label.as_deref()?;
                        let st = s.stats.as_ref()?;
                        let matches = st.get("Matches")
                            .and_then(|v| v.as_str()).unwrap_or("-");
                        let wins = st.get("Wins")
                            .and_then(|v| v.as_str()).unwrap_or("-");
                        let wr = st.get("Win Rate %")
                            .and_then(|v| v.as_str()).unwrap_or("-");
                        Some(format!("{}: M:{} W:{} WR:{}", label, matches, wins, wr))
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }).unwrap_or_default()
        } else { String::new() };

        let matches_text = self.match_history.as_ref().map(|h| {
            h.iter().take(20).map(|m| {
                let map = m.get("Map").cloned().unwrap_or_default();
                let score = m.get("Score").cloned().unwrap_or_default();
                let result = match m.get("Result").cloned().unwrap_or_default().as_str() {
                    "1" => "W", "0" => "L", _ => "?"
                };
                let kd = m.get("K/D Ratio").cloned().unwrap_or_default();
                format!("{} | {} | {} | KD:{}", map, score, result, kd)
            }).collect::<Vec<_>>().join("\n")
        }).unwrap_or_default();

        self.ai_loading = true;
        self.ai_partial = String::new();
        self.add_toast(Toast::info("AI Thinking..."));

        let (tx, rx) = mpsc::unbounded_channel();
        self.ai_rx = Some(rx);

        tokio::spawn(async move {
            if let Err(e) = crate::ai::analyze_player_streaming(&name, &lifetime, &maps, &matches_text, tx).await {
                eprintln!("AI streaming error: {}", e);
            }
        });
    }

    async fn fetch_page(&mut self, page: usize) -> AppResult<()> {
        let player_id = match &self.player_info {
            Some(p) => p.player_id.clone(),
            None => self.input.trim().to_string(),
        };

        self.loading_state = LoadingState::Loading;

        match self.api.get_match_history(&player_id, page * PAGE_SIZE, PAGE_SIZE).await {
            Ok(history) => {
                self.has_more_pages = history.len() >= PAGE_SIZE;
                    self.match_history = Some(history);
                    self.current_page = page;
                    self.selected_row = 0;
                    self.matches_sv = ScrollViewState::new();
                self.loading_state = LoadingState::Success;
            }
            Err(e) => {
                self.loading_state = LoadingState::Error(format!("Page load failed: {}", e));
            }
        }

        Ok(())
    }

    fn export_data(&mut self) {
        let name = self
            .player_info
            .as_ref()
            .map(|p| p.nickname.as_str())
            .unwrap_or("unknown");

        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let result = match self.selected_tab {
            0 => {
                let filename = format!("faceit_{}_lifetime_{}.json", name, ts);
                match &self.lifetime_stats {
                    Some(stats) => {
                        let json = serde_json::to_string_pretty(stats).unwrap_or_default();
                        std::fs::write(&filename, json).map(|_| filename)
                    }
                    None => Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No lifetime data")),
                }
            }
            1 => {
                let filename = format!("faceit_{}_matches_p{}_{}.json", name, self.current_page, ts);
                match &self.match_history {
                    Some(history) => {
                        let json = serde_json::to_string_pretty(history).unwrap_or_default();
                        std::fs::write(&filename, json).map(|_| filename)
                    }
                    None => Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No match data")),
                }
            }
            2 => {
                let filename = format!("faceit_{}_maps_{}.json", name, ts);
                match self.lifetime_stats.as_ref().and_then(|s| s.segments.as_ref()) {
                    Some(segments) => {
                        let json = serde_json::to_string_pretty(segments).unwrap_or_default();
                        std::fs::write(&filename, json).map(|_| filename)
                    }
                    None => Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No map data")),
                }
            }
            _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Invalid tab")),
        };

        match result {
            Ok(path) => {
                let full = std::env::current_dir()
                    .map(|d| d.join(&path))
                    .ok()
                    .and_then(|p| p.to_str().map(|s| s.to_string()))
                    .unwrap_or(path);
                self.add_toast(Toast::info(format!("Exported {}", full)));
            }
            Err(e) => self.add_toast(Toast::error(format!("Export failed: {}", e))),
        }
    }

    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        if self.show_theme_selector {
            match mouse.kind {
                MouseEventKind::ScrollUp => self.theme_sv.scroll_up(),
                MouseEventKind::ScrollDown => self.theme_sv.scroll_down(),
                _ => {}
            }
            return;
        }

        if self.show_match_popup {
            match mouse.kind {
                MouseEventKind::ScrollUp => self.match_popup_sv.scroll_up(),
                MouseEventKind::ScrollDown => self.match_popup_sv.scroll_down(),
                MouseEventKind::Down(_) => self.show_match_popup = false,
                _ => {}
            }
            return;
        }

        if self.show_map_popup {
            match mouse.kind {
                MouseEventKind::ScrollUp => self.map_popup_sv.scroll_up(),
                MouseEventKind::ScrollDown => self.map_popup_sv.scroll_down(),
                MouseEventKind::Down(_) => self.show_map_popup = false,
                _ => {}
            }
            return;
        }

        if self.ai_response.is_some() {
            match mouse.kind {
                MouseEventKind::ScrollUp => self.ai_sv.scroll_up(),
                MouseEventKind::ScrollDown => self.ai_sv.scroll_down(),
                _ => {}
            }
            return;
        }

        if self.ai_loading && !self.ai_partial.is_empty() {
            match mouse.kind {
                MouseEventKind::ScrollUp => self.ai_sv.scroll_up(),
                MouseEventKind::ScrollDown => self.ai_sv.scroll_down(),
                _ => {}
            }
            return;
        }

        if self.show_help {
            match mouse.kind {
                MouseEventKind::Down(_) => self.show_help = false,
                _ => {}
            }
            return;
        }

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                if self.loading_state != LoadingState::Success {
                    return;
                }
                match self.selected_tab {
                    0 => self.lifetime_sv.scroll_up(),
                    1 => self.matches_sv.scroll_up(),
                    2 => self.maps_sv.scroll_up(),
                    _ => {}
                }
            }
            MouseEventKind::ScrollDown => {
                if self.loading_state != LoadingState::Success {
                    return;
                }
                match self.selected_tab {
                    0 => self.lifetime_sv.scroll_down(),
                    1 => self.matches_sv.scroll_down(),
                    2 => self.maps_sv.scroll_down(),
                    _ => {}
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if self.show_help {
                    self.show_help = false;
                    return;
                }

                let my = mouse.row;
                let mx = mouse.column;

                // Click on input area → enter edit mode
                if let Some(r) = self.input_rect {
                    if my >= r.y && my < r.y + r.height && mx >= r.x && mx < r.x + r.width {
                        if self.input_mode == InputMode::Normal {
                            self.input_mode = InputMode::Editing;
                            self.history_index = None;
                        }
                        return;
                    }
                }

                // Click on player card area → toggle editing off if was editing
                if let Some(r) = self.player_card_rect {
                    if my >= r.y && my < r.y + r.height && mx >= r.x && mx < r.x + r.width {
                        if self.input_mode == InputMode::Editing {
                            self.input_mode = InputMode::Normal;
                        }
                        return;
                    }
                }

                // Click on toast area → dismiss toasts
                if !self.toasts.is_empty() {
                    let Some(rect) = self.last_content_rect else { return };
                    let toast_x = rect.x + rect.width.saturating_sub(38);
                    let toast_y = rect.y + 1;
                    let toast_h = self.toasts.len().min(2) as u16 + 2;
                    if my >= toast_y && my < toast_y + toast_h && mx >= toast_x && mx < rect.x + rect.width {
                        self.toasts.clear();
                        return;
                    }
                }

                if self.loading_state != LoadingState::Success {
                    return;
                }

                let Some(rect) = self.last_content_rect else { return };

                // Tab bar click
                if my == rect.y && rect.width >= 30 {
                    let third = rect.x + rect.width / 3;
                    let two_thirds = rect.x + 2 * rect.width / 3;
                    let new_tab = if (mx as u16) < third { 0 } else if (mx as u16) < two_thirds { 1 } else { 2 };
                    if new_tab != self.selected_tab {
                        self.selected_tab = new_tab;
                        self.selected_row = 0;
                        match new_tab {
                            0 => self.lifetime_sv.scroll_to_top(),
                            1 => self.matches_sv.scroll_to_top(),
                            2 => self.maps_sv.scroll_to_top(),
                            _ => {}
                        }
                    }
                    return;
                }

                // Table body click
                let body_start = rect.y + 3;
                if my >= body_start {
                    let rel = (my - body_start) as usize;
                    let offset = match self.selected_tab {
                        0 => self.lifetime_sv.offset().y,
                        1 => self.matches_sv.offset().y,
                        2 => self.maps_sv.offset().y,
                        _ => 0,
                    };
                    let new_row = rel + offset as usize;
                    let max = self.tab_item_count().saturating_sub(1);
                    if new_row <= max {
                        self.selected_row = new_row;
                    }
                }
            }
            _ => {}
        }
    }
}

fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}

fn value_to_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}
