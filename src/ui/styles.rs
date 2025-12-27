use ratatui::style::{Color, Modifier, Style};

// Help bar styles
/// Blue text for help descriptions
pub fn help() -> Style {
    Style::default().fg(Color::Blue)
}

/// Blue + Bold for keybinding letters
pub fn help_key() -> Style {
    Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::BOLD)
}

// Border styles
/// Green border for focused/active section
pub fn border_active() -> Style {
    Style::default().fg(Color::Green)
}

/// Default border for unfocused section
pub fn border_inactive() -> Style {
    Style::default()
}

// Title styles
/// Bold for active section titles
pub fn title_active() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

/// Default for inactive section titles
pub fn title_inactive() -> Style {
    Style::default()
}

// Selection styles
/// White on DarkGray for table row selection
pub fn row_highlight() -> Style {
    Style::default()
        .fg(Color::White)
        .bg(Color::DarkGray)
}

/// Yellow + Bold for selected list items
pub fn list_selected() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

// Feedback styles
/// Red text for error messages
pub fn error() -> Style {
    Style::default().fg(Color::Red)
}

/// Yellow text for warnings/status
pub fn warning() -> Style {
    Style::default().fg(Color::Yellow)
}

/// DarkGray for disabled items
pub fn disabled() -> Style {
    Style::default().fg(Color::DarkGray)
}

// Status indicator styles
/// Green for active/enabled states
pub fn status_active() -> Style {
    Style::default().fg(Color::Green)
}

// News indicator styles
/// Yellow for news items requiring attention (! indicator)
pub fn news_attention() -> Style {
    Style::default().fg(Color::Yellow)
}

/// Blue for news items related to installed packages (* indicator)
pub fn news_related() -> Style {
    Style::default().fg(Color::Blue)
}
