use ratatui::style::{Color, Modifier, Style};

pub struct AppTheme {
    source: opaline::Theme,
}

impl std::fmt::Debug for AppTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppTheme").finish()
    }
}

impl AppTheme {
    pub fn load(name: &str) -> Self {
        let theme = opaline::load_by_name(name)
            .or_else(|| opaline::load_by_name("flexoki-dark"))
            .unwrap_or_default();
        Self { source: theme }
    }

    fn to_color(c: opaline::OpalineColor) -> Color {
        Color::Rgb(c.r, c.g, c.b)
    }

    #[allow(dead_code)]
    fn to_style(s: opaline::OpalineStyle) -> Style {
        let mut style = Style::default();
        if let Some(fg) = s.fg {
            style = style.fg(Self::to_color(fg));
        }
        if let Some(bg) = s.bg {
            style = style.bg(Self::to_color(bg));
        }
        if s.bold { style = style.add_modifier(Modifier::BOLD); }
        if s.dim { style = style.add_modifier(Modifier::DIM); }
        if s.italic { style = style.add_modifier(Modifier::ITALIC); }
        if s.underline { style = style.add_modifier(Modifier::UNDERLINED); }
        if s.slow_blink { style = style.add_modifier(Modifier::SLOW_BLINK); }
        if s.rapid_blink { style = style.add_modifier(Modifier::RAPID_BLINK); }
        if s.reversed { style = style.add_modifier(Modifier::REVERSED); }
        if s.hidden { style = style.add_modifier(Modifier::HIDDEN); }
        if s.crossed_out { style = style.add_modifier(Modifier::CROSSED_OUT); }
        style
    }

    pub fn accent(&self) -> Color {
        Self::to_color(self.source.color("accent.primary"))
    }
    pub fn success(&self) -> Color {
        Self::to_color(self.source.color("success"))
    }
    pub fn error(&self) -> Color {
        Self::to_color(self.source.color("error"))
    }
    pub fn warn(&self) -> Color {
        Self::to_color(self.source.color("warning"))
    }
    pub fn fg(&self) -> Color {
        Self::to_color(self.source.color("text.primary"))
    }
    pub fn muted(&self) -> Color {
        Self::to_color(self.source.color("text.muted"))
    }
    pub fn bg(&self) -> Color {
        Self::to_color(self.source.color("bg.base"))
    }
    pub fn panel_bg(&self) -> Color {
        Self::to_color(self.source.color("bg.panel"))
    }
    pub fn highlight_bg(&self) -> Color {
        Self::to_color(self.source.color("bg.selection"))
    }

    #[allow(dead_code)]
    pub fn accent_style(&self) -> Style {
        Self::to_style(self.source.style("keyword"))
    }
    #[allow(dead_code)]
    pub fn success_style(&self) -> Style {
        Self::to_style(self.source.style("success"))
    }
    #[allow(dead_code)]
    pub fn error_style(&self) -> Style {
        Self::to_style(self.source.style("error"))
    }
}
