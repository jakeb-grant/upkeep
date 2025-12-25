# Hyprpier TUI Style Guide

This document defines the visual styling conventions for the Hyprpier terminal user interface built with [Ratatui](https://ratatui.rs/).

## Color Palette

The TUI uses a minimal, purposeful color palette from Ratatui's `Color` enum:

| Color | Usage | Context |
|-------|-------|---------|
| **Blue** | Help text and keybindings | Help bars at bottom of screens |
| **Yellow** | Focus/selection, warnings | Active inputs, selected items, warning dialogs |
| **Green** | Active/enabled states | Active section borders, security "none", auto-switch enabled |
| **Red** | Errors, danger | Error messages, danger dialogs (delete), security "secure" |
| **DarkGray** | Disabled/inactive | Disabled items, dimmed text, auto-switch disabled |
| **White** | Highlighted row foreground | Table row selection (on DarkGray background) |
| **Gray** | Unknown states | Unknown security mode |

### Semantic Color Meanings

- **Green** = safe, active, enabled, connected
- **Yellow** = focused, selected, warning, needs attention
- **Red** = danger, error, restricted
- **DarkGray** = disabled, inactive, unavailable

## Style Functions

All styles are centralized in `src/tui/styles.rs`. Use these functions instead of creating inline styles:

```rust
use super::styles;

// Help bar
styles::help()           // Blue text for help descriptions
styles::help_key()       // Blue + Bold for keybinding letters

// Headers
styles::header_active()  // Yellow + Bold for active table headers
styles::header_inactive()// Default for inactive table headers

// Borders
styles::border_active()  // Green border for focused section
styles::border_inactive()// Default border for unfocused section

// Titles
styles::title_active()   // Bold for active section titles
styles::title_inactive() // Default for inactive section titles

// Selection
styles::row_highlight()  // White on DarkGray for table row selection
styles::list_selected()  // Yellow + Bold for selected list items

// Input
styles::input_focused()  // Yellow border for focused input fields

// Feedback
styles::error()          // Red text for error messages
styles::warning()        // Yellow text for warnings/status
styles::disabled()       // DarkGray for disabled items

// Layout
styles::page_title()     // Bold for page titles (centered)
```

## Active/Inactive States

The TUI uses a two-state system for focusable sections:

### Active State
When a section has focus:
- Border: `styles::border_active()` (Green)
- Title: `styles::title_active()` (Bold)
- Header: `styles::header_active()` (Yellow + Bold)
- Row highlight: `styles::row_highlight()` (White on DarkGray)
- Selection indicator: `">> "`

### Inactive State
When a section does not have focus:
- Border: `styles::border_inactive()` (Default)
- Title: `styles::title_inactive()` (Default)
- Header: `styles::header_inactive()` (Default)
- Row highlight: `Style::default()` (None)
- Selection indicator: `"   "` (spaces)

### Example Pattern

```rust
let is_active = state.section == Section::Connected;

let table = Table::new(rows, widths)
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Section Name ")
            .title_style(if is_active { styles::title_active() } else { styles::title_inactive() })
            .border_style(if is_active { styles::border_active() } else { styles::border_inactive() }),
    )
    .row_highlight_style(if is_active { styles::row_highlight() } else { Style::default() })
    .highlight_symbol(if is_active { ">> " } else { "   " });
```

## Help Bars

Help bars appear at the bottom of each screen and follow a consistent format.

### Structure
- 2 lines of centered text
- Primary actions on line 1
- Navigation and exit on line 2
- No border/box around help text

### Format Pattern
```
[key] Action | [key] Action | [key] Action
[key] Nav | [key] Nav | [key] Quit/Back
```

### Separator
Items are separated by ` | ` (space-pipe-space). The pipe and surrounding spaces use `styles::help()` (regular blue), not bold:

```rust
Span::styled(" | ", styles::help())
```

### Key Styling
Keys use bold blue, descriptions use regular blue:

```rust
Line::from(vec![
    Span::styled("n", styles::help_key()),
    Span::styled(" New | ", styles::help()),
    Span::styled("e", styles::help_key()),
    Span::styled(" Edit | ", styles::help()),
    Span::styled("q", styles::help_key()),
    Span::styled(" Quit", styles::help()),
])
```

### Navigation Keys
Standard navigation keybindings across all screens:
- `j,↓` / `k,↑` - Down/Up navigation
- `h,←` / `l,→` - Left/Right or horizontal actions
- `Tab` - Switch sections/fields
- `Enter` / `↵` - Confirm/Edit
- `Esc` - Cancel/Back
- `q` - Quit (only on main screen)

## Input Fields

### Text Input States

| State | Border Style | Cursor |
|-------|--------------|--------|
| Unfocused | Default | None |
| Focused | `styles::input_focused()` (Yellow) | None |
| Input mode | `styles::input_focused()` (Yellow) | `_` appended |

### Example
```rust
let name_style = if state.focused_field == 0 {
    styles::input_focused()
} else {
    Style::default()
};

let cursor = if state.input_mode && state.focused_field == 0 { "_" } else { "" };

let name_block = Block::default()
    .borders(Borders::ALL)
    .title(" Name ")
    .border_style(name_style);

let name_para = Paragraph::new(format!("{}{}", state.name_input, cursor))
    .block(name_block);
```

## Confirmation Dialogs

Dialogs are centered overlays with semantic styling.

### Dialog Styles

| Style | Border Color | Use Case |
|-------|--------------|----------|
| `ConfirmStyle::Danger` | Red | Delete, destructive actions |
| `ConfirmStyle::Warning` | Yellow | Overwrite, unlink, reassign |

### Structure
- Fixed dimensions: 55 x 8 characters
- Centered on screen
- Uses `Clear` widget to erase background
- Format: `[y] Yes  [n] No`

```rust
let border_color = match dialog.style {
    ConfirmStyle::Danger => Color::Red,
    ConfirmStyle::Warning => Color::Yellow,
};

let block = Block::default()
    .title(format!(" {} ", dialog.title))
    .borders(Borders::ALL)
    .border_style(Style::default().fg(border_color));
```

## Tables

### Column Layout
Use percentage-based constraints for responsive layouts:

```rust
Table::new(
    rows,
    [
        Constraint::Percentage(20),  // Name
        Constraint::Percentage(40),  // Description
        Constraint::Percentage(15),  // Count
        Constraint::Percentage(25),  // Status
    ],
)
```

### Header Style
Always style headers based on section active state:

```rust
let header = Row::new(vec![
    Cell::from("Column").style(styles::header_active()),
    // ...
])
.height(1);
```

### Row Highlight
Use the built-in highlight system:

```rust
.row_highlight_style(styles::row_highlight())
.highlight_symbol(">> ")
```

## Status Indicators

### Security Mode Colors
Used in Thunderbolt manager:

| Mode | Color | Meaning |
|------|-------|---------|
| `none` | Green | No security restrictions |
| `user` | Yellow | User approval required |
| `secure` | Red | Secure boot verification |
| Unknown | Gray | Cannot determine |

### Auto-switch Status

| State | Color | Text |
|-------|-------|------|
| Enabled | Green | `enabled` |
| Disabled | DarkGray | `disabled` |

### Profile Status Labels
- `active` - Currently applied profile
- `docked` - Linked to a dock
- `undocked` - Fallback for undocked state
- `error` - Failed to load (styled with `styles::error()`)

## Canvas/Preview Rendering

For visual previews like monitor arrangement:

### Monitor Colors
- Normal monitors: `Color::Green`
- Selected monitor: `Color::Yellow`

### Selection Indication
- Selected monitor is drawn last (on top)
- Slight inset (1px) to show clear boundaries
- Both rectangle and label use selection color

```rust
// Non-selected: green
ctx.draw(&Rectangle { color: Color::Green, ... });
ctx.print(x, y, Line::styled(name, Style::default().fg(Color::Green)));

// Selected: yellow with inset
ctx.draw(&Rectangle { color: Color::Yellow, x: pm.x + 1.0, ... });
ctx.print(x, y, Line::styled(name, Style::default().fg(Color::Yellow)));
```

## Layout Patterns

### Standard Screen Layout

```rust
Layout::vertical([
    Constraint::Length(1),    // Title (centered, bold)
    Constraint::Length(1),    // Status bar (optional)
    Constraint::Min(N),       // Main content
    Constraint::Length(H),    // Secondary content (optional)
    Constraint::Length(1),    // Error message (if any, 0 if none)
    Constraint::Length(2),    // Help bar
])
```

### Conditional Rows
Use 0-height constraints for optional elements:

```rust
Constraint::Length(if has_error { 1 } else { 0 })
```

### Title Alignment
Page titles are always centered and bold:

```rust
Paragraph::new("Page Title")
    .style(styles::page_title())
    .alignment(ratatui::layout::Alignment::Center)
```

## Modifiers

Only these modifiers are used:

| Modifier | Usage |
|----------|-------|
| `Modifier::BOLD` | Help keys, active headers, active titles, selected items |

No italic, underline, or other modifiers are used to maintain terminal compatibility.

## Adding New Styles

When adding new UI elements:

1. Check if an existing style function applies
2. If not, add a new function to `src/tui/styles.rs`
3. Include a doc comment explaining the use case
4. Follow the existing color semantics

```rust
/// Style for new element type (description)
pub fn new_element() -> Style {
    Style::default().fg(Color::...).add_modifier(Modifier::...)
}
```

## Summary: Quick Reference

### What Color to Use

| Situation | Color |
|-----------|-------|
| User can interact now | Yellow |
| Something is active/enabled | Green |
| Error occurred | Red |
| Destructive action | Red |
| Warning/needs attention | Yellow |
| Disabled/unavailable | DarkGray |
| Help text | Blue |
| Selected row background | DarkGray |
| Selected row text | White |

### What Style Function to Use

| Element | Function |
|---------|----------|
| Help text | `styles::help()` |
| Help keybinding | `styles::help_key()` |
| Active section border | `styles::border_active()` |
| Active section title | `styles::title_active()` |
| Active table header | `styles::header_active()` |
| Selected table row | `styles::row_highlight()` |
| Selected list item | `styles::list_selected()` |
| Focused input | `styles::input_focused()` |
| Page title | `styles::page_title()` |
| Error message | `styles::error()` |
| Warning message | `styles::warning()` |
| Disabled element | `styles::disabled()` |
