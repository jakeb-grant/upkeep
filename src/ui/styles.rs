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

// Header styles
/// Yellow + Bold for active table headers
pub fn header_active() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

/// Default for inactive table headers
pub fn header_inactive() -> Style {
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

// Input styles
/// Yellow border for focused input fields
pub fn input_focused() -> Style {
    Style::default().fg(Color::Yellow)
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

// Layout styles
/// Bold for page titles (centered)
pub fn page_title() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

// Status indicator styles
/// Green for active/enabled states
pub fn status_active() -> Style {
    Style::default().fg(Color::Green)
}

/// Yellow for focused/selected states
pub fn status_focused() -> Style {
    Style::default().fg(Color::Yellow)
}

/// Red for errors/danger
pub fn status_danger() -> Style {
    Style::default().fg(Color::Red)
}
