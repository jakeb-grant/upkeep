use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::app::ConfirmationState;

use super::styles;

pub fn draw_confirmation(frame: &mut Frame, state: &ConfirmationState, area: Rect) {
    // Calculate dialog size based on content
    let max_item_width = state
        .items
        .iter()
        .map(|s| s.len())
        .max()
        .unwrap_or(0)
        .max(state.title.len())
        .max(state.message.len())
        .max(30);

    let dialog_width = (max_item_width as u16 + 8).min(area.width.saturating_sub(4));
    let item_lines = state.items.len().min(15) as u16; // Cap at 15 visible items
    let dialog_height = (item_lines + 8).min(area.height.saturating_sub(2));

    let dialog_area = centered_rect(dialog_width, dialog_height, area);

    // Clear background
    frame.render_widget(Clear, dialog_area);

    // Build content lines
    let mut lines = vec![
        Line::from(Span::styled(&state.title, styles::title_active())),
        Line::from(""),
    ];

    // Show items (with scroll indicator if too many)
    let max_visible = (dialog_height.saturating_sub(8)) as usize;
    let items_to_show = if state.items.len() > max_visible {
        &state.items[..max_visible]
    } else {
        &state.items
    };

    for item in items_to_show {
        lines.push(Line::from(format!("  {}", item)));
    }

    if state.items.len() > max_visible {
        lines.push(Line::from(Span::styled(
            format!("  ... and {} more", state.items.len() - max_visible),
            styles::disabled(),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        state.message.clone(),
        styles::warning(),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("[Enter/y]", styles::help_key()),
        Span::styled(" Confirm  ", styles::help()),
        Span::styled("[Esc/n]", styles::help_key()),
        Span::styled(" Cancel", styles::help()),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(styles::border_active())
        .title(" Confirm ");

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, dialog_area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}
