mod styles;

use crate::app::{App, LoadingState, Tab};
use crate::updates::{format_short_date, NewsInfo, PackageInfo};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame,
};

fn truncate_with_ellipsis(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        format!("{:<width$}", s, width = max_len)
    } else {
        format!("{:<width$}", format!("{}...", &s[..max_len.saturating_sub(3)]), width = max_len)
    }
}

fn draw_empty_state(frame: &mut Frame, title: &str, message: &str, is_active: bool, area: Rect) {
    let paragraph = Paragraph::new(message)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(if is_active {
                    styles::title_active()
                } else {
                    styles::title_inactive()
                })
                .border_style(if is_active {
                    styles::border_active()
                } else {
                    styles::border_inactive()
                }),
        )
        .style(styles::disabled());

    frame.render_widget(paragraph, area);
}

fn draw_filter_bar(frame: &mut Frame, filter_text: &str, filter_mode: bool, match_count: usize, area: Rect) {
    let filter_display = if filter_mode {
        format!(" Filter: {}█", filter_text)
    } else {
        format!(" Filter: {} ({} matches)", filter_text, match_count)
    };
    let filter_bar = Paragraph::new(filter_display).style(if filter_mode {
        styles::warning()
    } else {
        styles::disabled()
    });
    frame.render_widget(filter_bar, area);
}

fn draw_info_pane(frame: &mut Frame, info: Option<&PackageInfo>, area: Rect) {
    let content = if let Some(info) = info {
        // Line 1: name version (repository)
        let repo_display = if info.repository.is_empty() {
            String::new()
        } else {
            format!("({})", info.repository)
        };
        let line1 = Line::from(vec![
            Span::styled(&info.name, styles::title_active()),
            Span::raw(" "),
            Span::styled(&info.version, styles::status_active()),
            Span::raw(" "),
            Span::styled(repo_display, styles::disabled()),
        ]);

        // Line 2: description (truncated if needed)
        let line2 = Line::from(Span::raw(&info.description));

        // Line 3: size + install info
        let install_info = match (&info.install_date, &info.install_reason) {
            (Some(date), Some(reason)) => format!(" | {} | {}", date, reason),
            (Some(date), None) => format!(" | {}", date),
            _ => String::new(),
        };
        let line3 = Line::from(vec![
            Span::styled("Size: ", styles::disabled()),
            Span::styled(&info.size, styles::status_active()),
            Span::styled(install_info, styles::disabled()),
        ]);

        // Line 4: URL
        let line4 = if let Some(url) = &info.url {
            Line::from(vec![
                Span::styled("URL: ", styles::disabled()),
                Span::styled(url.as_str(), styles::status_active()),
            ])
        } else {
            Line::from(Span::styled("URL: ", styles::disabled()))
        };

        // Line 5: Built date
        let line5 = if let Some(build_date) = &info.build_date {
            Line::from(vec![
                Span::styled("Built: ", styles::disabled()),
                Span::styled(build_date.as_str(), styles::status_active()),
            ])
        } else {
            Line::from(Span::styled("Built: ", styles::disabled()))
        };

        // Line 6: Required By
        let line6 = if !info.required_by.is_empty() {
            let pkgs = truncate_with_ellipsis(&info.required_by.join(", "), 60);
            Line::from(vec![
                Span::styled("Required by: ", styles::disabled()),
                Span::styled(pkgs, styles::status_active()),
            ])
        } else {
            Line::from(Span::styled("Required by: None", styles::disabled()))
        };

        // Line 7: Optional For
        let line7 = if !info.optional_for.is_empty() {
            let pkgs = truncate_with_ellipsis(&info.optional_for.join(", "), 60);
            Line::from(vec![
                Span::styled("Optional for: ", styles::disabled()),
                Span::styled(pkgs, styles::status_active()),
            ])
        } else {
            Line::from(Span::styled("Optional for: None", styles::disabled()))
        };

        // Line 8: Maintainer + Votes (AUR only)
        let line8 = if info.maintainer.is_some() || info.votes.is_some() {
            let mut spans = Vec::new();
            if let Some(maintainer) = &info.maintainer {
                spans.push(Span::styled("Maintainer: ", styles::disabled()));
                spans.push(Span::styled(maintainer.as_str(), styles::status_active()));
            }
            if let Some(votes) = &info.votes {
                if !spans.is_empty() {
                    spans.push(Span::styled(" | ", styles::disabled()));
                }
                spans.push(Span::styled("Votes: ", styles::disabled()));
                spans.push(Span::styled(votes.to_string(), styles::status_active()));
            }
            Line::from(spans)
        } else {
            Line::from("")
        };

        // Filter out empty lines
        vec![line1, line2, line3, line4, line5, line6, line7, line8]
            .into_iter()
            .filter(|line| !line.spans.is_empty())
            .collect()
    } else {
        vec![Line::from(Span::styled(
            "No package info available",
            styles::disabled(),
        ))]
    };

    let paragraph = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Info ")
            .title_style(styles::title_inactive())
            .border_style(styles::border_inactive()),
    );

    frame.render_widget(paragraph, area);
}

fn format_package_name(name: &str, source_label: &str, total_width: usize) -> String {
    let combined = format!("{}{}", name, source_label);
    if combined.len() <= total_width {
        format!("{:<width$}", combined, width = total_width)
    } else {
        // Truncate name, preserve source label
        let available_for_name = total_width.saturating_sub(source_label.len()).saturating_sub(3);
        let truncated_name = &name[..available_for_name.min(name.len())];
        format!("{:<width$}", format!("{}...{}", truncated_name, source_label), width = total_width)
    }
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Header + tabs
        Constraint::Length(1), // Status bar
        Constraint::Min(0),    // Content
        Constraint::Length(2), // Help bar
    ])
    .split(frame.area());

    draw_header(frame, app, chunks[0]);
    draw_status(frame, app, chunks[1]);
    draw_content(frame, app, chunks[2]);
    draw_help(frame, app, chunks[3]);
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let titles = vec!["Updates", "Installed", "Orphans", "Rebuilds", "Search", "News"];
    let selected = match app.tab {
        Tab::Updates => 0,
        Tab::Installed => 1,
        Tab::Orphans => 2,
        Tab::Rebuilds => 3,
        Tab::Search => 4,
        Tab::News => 5,
    };

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(styles::border_active())
                .title(" upkeep ")
                .title_style(styles::title_active()),
        )
        .select(selected)
        .style(Style::default())
        .highlight_style(styles::list_selected());

    frame.render_widget(tabs, area);
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let width = area.width as usize;

    let loading = app.loading == LoadingState::Loading;
    let pac = app.pacman_count();
    let aur = app.aur_count();
    let inst = app.installed_count();
    let inst_aur = app.installed_aur_count();
    let orph = app.orphan_count();
    let rebuild = app.rebuild_issues.len();

    let pac_style = if pac > 0 { styles::warning() } else { styles::status_active() };
    let aur_style = if aur > 0 { styles::warning() } else { styles::status_active() };
    let orph_style = if orph > 0 { styles::warning() } else { styles::status_active() };
    let rebuild_style = if rebuild > 0 { styles::error() } else { styles::status_active() };

    let status = if width >= 100 {
        // Wide: full labels
        let loading_indicator = if loading { " [loading...]" } else { "" };
        Line::from(vec![
            Span::raw(" Pacman: "),
            Span::styled(format!("{} updates", pac), pac_style),
            Span::styled(" | ", styles::disabled()),
            Span::raw("AUR: "),
            Span::styled(format!("{} updates", aur), aur_style),
            Span::styled(" | ", styles::disabled()),
            Span::raw("Installed: "),
            Span::styled(format!("{}", inst), styles::status_active()),
            Span::styled(format!(" ({} AUR)", inst_aur), styles::disabled()),
            Span::styled(" | ", styles::disabled()),
            Span::raw("Orphans: "),
            Span::styled(format!("{}", orph), orph_style),
            Span::styled(" | ", styles::disabled()),
            Span::raw("Rebuilds: "),
            Span::styled(format!("{} issues", rebuild), rebuild_style),
            Span::styled(loading_indicator, styles::warning()),
        ])
    } else if width >= 60 {
        // Medium: abbreviated labels
        let loading_indicator = if loading { " [...]" } else { "" };
        Line::from(vec![
            Span::raw(" Pac: "),
            Span::styled(format!("{}", pac), pac_style),
            Span::styled(" | ", styles::disabled()),
            Span::raw("AUR: "),
            Span::styled(format!("{}", aur), aur_style),
            Span::styled(" | ", styles::disabled()),
            Span::raw("Inst: "),
            Span::styled(format!("{}", inst), styles::status_active()),
            Span::styled(format!(" ({})", inst_aur), styles::disabled()),
            Span::styled(" | ", styles::disabled()),
            Span::raw("Orph: "),
            Span::styled(format!("{}", orph), orph_style),
            Span::styled(" | ", styles::disabled()),
            Span::raw("Reb: "),
            Span::styled(format!("{}", rebuild), rebuild_style),
            Span::styled(loading_indicator, styles::warning()),
        ])
    } else {
        // Narrow: minimal
        let loading_indicator = if loading { " *" } else { "" };
        Line::from(vec![
            Span::raw(" P:"),
            Span::styled(format!("{}", pac), pac_style),
            Span::raw(" A:"),
            Span::styled(format!("{}", aur), aur_style),
            Span::raw(" I:"),
            Span::styled(format!("{}", inst), styles::status_active()),
            Span::raw(" O:"),
            Span::styled(format!("{}", orph), orph_style),
            Span::raw(" R:"),
            Span::styled(format!("{}", rebuild), rebuild_style),
            Span::styled(loading_indicator, styles::warning()),
        ])
    };

    let paragraph = Paragraph::new(status);
    frame.render_widget(paragraph, area);
}

fn draw_content(frame: &mut Frame, app: &mut App, area: Rect) {
    match app.tab {
        Tab::Updates => draw_updates(frame, app, area),
        Tab::Installed => draw_installed(frame, app, area),
        Tab::Orphans => draw_orphans(frame, app, area),
        Tab::Rebuilds => draw_rebuilds(frame, app, area),
        Tab::Search => draw_search(frame, app, area),
        Tab::News => draw_news(frame, app, area),
    }
}

fn draw_updates(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_active = app.tab == Tab::Updates;

    // Split area for info pane if visible
    let (main_area, info_area) = if app.show_info_pane {
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(10)]).split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    // Split main area for filter bar if filtering
    let (filter_area, list_area) = if app.filter_mode || !app.filter_text.is_empty() {
        let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(main_area);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, main_area)
    };

    // Collect filtered packages into owned data
    let filtered: Vec<(usize, bool, String, String, String, &'static str)> = app
        .filtered_updates()
        .into_iter()
        .map(|(idx, pkg)| {
            (
                idx,
                pkg.selected,
                pkg.name.clone(),
                pkg.old_version.clone(),
                pkg.new_version.clone(),
                pkg.source_label(),
            )
        })
        .collect();
    let filtered_count = filtered.len();

    // Draw filter bar
    if let Some(filter_area) = filter_area {
        draw_filter_bar(frame, &app.filter_text, app.filter_mode, filtered_count, filter_area);
    }

    if app.packages.is_empty() {
        let message = if app.loading == LoadingState::Loading {
            "Checking for updates..."
        } else {
            "No updates available"
        };
        draw_empty_state(frame, " Packages ", message, is_active, list_area);
        return;
    }

    if filtered_count == 0 && !app.filter_text.is_empty() {
        draw_empty_state(frame, " Packages ", "No packages match filter", is_active, list_area);
        return;
    }

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(filter_idx, (_, selected, name, old_version, new_version, source))| {
            let is_cursor = app.list_state.selected() == Some(filter_idx);
            let checkbox = if *selected { "[x]" } else { "[ ]" };

            let line = Line::from(vec![
                Span::styled(
                    format!("{} ", checkbox),
                    if *selected {
                        styles::status_active()
                    } else {
                        styles::disabled()
                    },
                ),
                Span::styled(
                    format_package_name(name, source, 30),
                    if is_cursor && is_active {
                        styles::row_highlight()
                    } else {
                        Style::default()
                    },
                ),
                Span::raw(" "),
                Span::styled(truncate_with_ellipsis(old_version, 14), styles::disabled()),
                Span::styled(" -> ", styles::disabled()),
                Span::styled(new_version, styles::status_active()),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Packages ")
                .title_style(if is_active {
                    styles::title_active()
                } else {
                    styles::title_inactive()
                })
                .border_style(if is_active {
                    styles::border_active()
                } else {
                    styles::border_inactive()
                }),
        )
        .highlight_style(styles::row_highlight())
        .highlight_symbol(if is_active { ">> " } else { "   " });

    frame.render_stateful_widget(list, list_area, &mut app.list_state);

    // Draw info pane if visible
    if let Some(info_area) = info_area {
        draw_info_pane(frame, app.cached_pkg_info.as_ref(), info_area);
    }
}

fn draw_installed(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_active = app.tab == Tab::Installed;

    // Split area for info pane if visible
    let (main_area, info_area) = if app.show_info_pane {
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(10)]).split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    // Split main area for filter bar if filtering
    let (filter_area, list_area) = if app.filter_mode || !app.filter_text.is_empty() {
        let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(main_area);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, main_area)
    };

    // Collect filtered packages into owned data to avoid borrow conflicts
    let filtered: Vec<(usize, bool, String, String, &'static str)> = app
        .filtered_installed()
        .into_iter()
        .map(|(idx, pkg)| (idx, pkg.selected, pkg.name.clone(), pkg.version.clone(), pkg.source_label()))
        .collect();
    let filtered_count = filtered.len();

    // Draw filter bar
    if let Some(filter_area) = filter_area {
        draw_filter_bar(frame, &app.filter_text, app.filter_mode, filtered_count, filter_area);
    }

    if app.installed_packages.is_empty() {
        let message = if app.loading == LoadingState::Loading {
            "Loading installed packages..."
        } else {
            "No explicitly installed packages found"
        };
        draw_empty_state(frame, " Installed Packages ", message, is_active, list_area);
        return;
    }

    if filtered_count == 0 && !app.filter_text.is_empty() {
        draw_empty_state(frame, " Installed Packages ", "No packages match filter", is_active, list_area);
        return;
    }

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(filter_idx, (_, selected, name, version, source))| {
            let is_cursor = app.installed_list_state.selected() == Some(filter_idx);
            let checkbox = if *selected { "[x]" } else { "[ ]" };

            let line = Line::from(vec![
                Span::styled(
                    format!("{} ", checkbox),
                    if *selected {
                        styles::status_active()
                    } else {
                        styles::disabled()
                    },
                ),
                Span::styled(
                    format_package_name(name, source, 36),
                    if is_cursor && is_active {
                        styles::row_highlight()
                    } else {
                        Style::default()
                    },
                ),
                Span::raw(" "),
                Span::styled(version, styles::disabled()),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Installed Packages ")
                .title_style(if is_active {
                    styles::title_active()
                } else {
                    styles::title_inactive()
                })
                .border_style(if is_active {
                    styles::border_active()
                } else {
                    styles::border_inactive()
                }),
        )
        .highlight_style(styles::row_highlight())
        .highlight_symbol(if is_active { ">> " } else { "   " });

    frame.render_stateful_widget(list, list_area, &mut app.installed_list_state);

    // Draw info pane if visible
    if let Some(info_area) = info_area {
        draw_info_pane(frame, app.cached_pkg_info.as_ref(), info_area);
    }
}

fn draw_orphans(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_active = app.tab == Tab::Orphans;

    // Split area for info pane if visible
    let (list_area, info_area) = if app.show_info_pane {
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(10)]).split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    if app.orphan_packages.is_empty() {
        let message = if app.loading == LoadingState::Loading {
            "Checking for orphan packages..."
        } else {
            "No orphan packages found"
        };
        draw_empty_state(frame, " Orphan Packages ", message, is_active, list_area);
        if let Some(info_area) = info_area {
            draw_info_pane(frame, app.cached_pkg_info.as_ref(), info_area);
        }
        return;
    }

    let items: Vec<ListItem> = app
        .orphan_packages
        .iter()
        .enumerate()
        .map(|(idx, pkg)| {
            let is_selected = app.orphans_list_state.selected() == Some(idx);
            let checkbox = if pkg.selected { "[x]" } else { "[ ]" };

            let line = Line::from(vec![
                Span::styled(
                    format!("{} ", checkbox),
                    if pkg.selected {
                        styles::status_active()
                    } else {
                        styles::disabled()
                    },
                ),
                Span::styled(
                    format_package_name(&pkg.name, pkg.source_label(), 36),
                    if is_selected && is_active {
                        styles::row_highlight()
                    } else {
                        Style::default()
                    },
                ),
                Span::raw(" "),
                Span::styled(&pkg.version, styles::disabled()),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Orphan Packages ")
                .title_style(if is_active {
                    styles::title_active()
                } else {
                    styles::title_inactive()
                })
                .border_style(if is_active {
                    styles::border_active()
                } else {
                    styles::border_inactive()
                }),
        )
        .highlight_style(styles::row_highlight())
        .highlight_symbol(if is_active { ">> " } else { "   " });

    frame.render_stateful_widget(list, list_area, &mut app.orphans_list_state);

    // Draw info pane if visible
    if let Some(info_area) = info_area {
        draw_info_pane(frame, app.cached_pkg_info.as_ref(), info_area);
    }
}

fn draw_rebuilds(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_active = app.tab == Tab::Rebuilds;

    // Split area for info pane if visible
    let (list_area, info_area) = if app.show_info_pane {
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(10)]).split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    if app.rebuild_issues.is_empty() {
        let message = if app.loading == LoadingState::Loading {
            "Checking for rebuild issues..."
        } else if app.rebuild_checks.is_empty() {
            "No rebuild checks configured\nAdd checks to ~/.config/upkeep/checks.toml"
        } else {
            "No rebuild issues detected"
        };
        draw_empty_state(frame, " Rebuild Issues ", message, is_active, list_area);
        if let Some(info_area) = info_area {
            draw_info_pane(frame, app.cached_pkg_info.as_ref(), info_area);
        }
        return;
    }

    let items: Vec<ListItem> = app
        .rebuild_issues
        .iter()
        .enumerate()
        .map(|(idx, issue)| {
            let is_selected = app.rebuilds_list_state.selected() == Some(idx);
            let checkbox = if issue.selected { "[x]" } else { "[ ]" };

            let line = Line::from(vec![
                Span::styled(
                    format!("{} ", checkbox),
                    if issue.selected {
                        styles::status_active()
                    } else {
                        styles::disabled()
                    },
                ),
                Span::styled(
                    &issue.name,
                    if is_selected && is_active {
                        styles::row_highlight()
                    } else {
                        styles::error()
                    },
                ),
                Span::styled(" - needs rebuild", styles::disabled()),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Rebuild Issues ")
                .title_style(if is_active {
                    styles::title_active()
                } else {
                    styles::title_inactive()
                })
                .border_style(if is_active {
                    styles::border_active()
                } else {
                    styles::border_inactive()
                }),
        )
        .highlight_style(styles::row_highlight())
        .highlight_symbol(if is_active { ">> " } else { "   " });

    frame.render_stateful_widget(list, list_area, &mut app.rebuilds_list_state);

    // Draw info pane if visible
    if let Some(info_area) = info_area {
        draw_info_pane(frame, app.cached_pkg_info.as_ref(), info_area);
    }
}

fn draw_search(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_active = app.tab == Tab::Search;

    // Split area for info pane if visible
    let (main_area, info_area) = if app.show_info_pane {
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(10)]).split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    // Split main area for search bar
    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(main_area);
    let search_area = chunks[0];
    let list_area = chunks[1];

    // Draw search bar
    let search_display = format!(" Search: {}█", app.search_query);
    let search_bar = Paragraph::new(search_display).style(styles::warning());
    frame.render_widget(search_bar, search_area);

    // Draw results
    if app.search_results.is_empty() {
        let message = if app.search_query.len() < 2 {
            "Type to search packages..."
        } else if app.search_loading {
            "Searching..."
        } else {
            "No results found"
        };
        draw_empty_state(frame, " Search Results ", message, is_active, list_area);
    } else {
        let items: Vec<ListItem> = app
            .search_results
            .iter()
            .enumerate()
            .map(|(idx, result)| {
                let is_selected = app.search_list_state.selected() == Some(idx);
                let checkbox = if result.selected {
                    "[x]"
                } else if result.installed {
                    "[=]"
                } else {
                    "[ ]"
                };

                let source_label = format!(" ({})", result.repository);
                let line = Line::from(vec![
                    Span::styled(
                        format!("{} ", checkbox),
                        if result.selected {
                            styles::status_active()
                        } else {
                            styles::disabled()
                        },
                    ),
                    Span::styled(
                        format_package_name(&result.name, &source_label, 36),
                        if is_selected && is_active {
                            styles::row_highlight()
                        } else if result.installed {
                            styles::disabled()
                        } else {
                            Style::default()
                        },
                    ),
                    Span::raw(" "),
                    Span::styled(&result.version, styles::disabled()),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Search Results ({}) ", app.search_results.len()))
                    .title_style(if is_active {
                        styles::title_active()
                    } else {
                        styles::title_inactive()
                    })
                    .border_style(if is_active {
                        styles::border_active()
                    } else {
                        styles::border_inactive()
                    }),
            )
            .highlight_style(styles::row_highlight())
            .highlight_symbol(if is_active { ">> " } else { "   " });

        frame.render_stateful_widget(list, list_area, &mut app.search_list_state);
    }

    // Draw info pane if visible
    if let Some(info_area) = info_area {
        draw_info_pane(frame, app.cached_pkg_info.as_ref(), info_area);
    }
}

fn draw_news(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_active = app.tab == Tab::News;

    // Split area for info pane if visible (half screen for article content)
    let (list_area, info_area) = if app.show_info_pane {
        let chunks = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    if app.news_items.is_empty() {
        let message = if app.news_loading {
            "Loading Arch Linux news..."
        } else if app.news_error {
            "Failed to fetch news (press r to retry)"
        } else {
            "No news items available"
        };
        draw_empty_state(frame, " Arch News ", message, is_active, list_area);
        if let Some(info_area) = info_area {
            draw_news_info_pane(frame, app.cached_news_info.as_ref(), app.news_scroll, info_area);
        }
        return;
    }

    let items: Vec<ListItem> = app
        .news_items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let is_selected = app.news_list_state.selected() == Some(idx);

            // Build indicator: * for related, ! for attention
            let indicator = match (item.requires_attention, !item.related_packages.is_empty()) {
                (true, true) => "*!",
                (true, false) => " !",
                (false, true) => "* ",
                (false, false) => "  ",
            };

            // Date in short format
            let date_short = format_short_date(&item.pub_date);

            let line = Line::from(vec![
                // * indicator (blue)
                Span::styled(
                    &indicator[0..1],
                    if !item.related_packages.is_empty() {
                        styles::news_related()
                    } else {
                        Style::default()
                    },
                ),
                // ! indicator (yellow)
                Span::styled(
                    &indicator[1..2],
                    if item.requires_attention {
                        styles::news_attention()
                    } else {
                        Style::default()
                    },
                ),
                Span::raw(" "),
                // Date
                Span::styled(format!("{:<6} ", date_short), styles::disabled()),
                // Title
                Span::styled(
                    truncate_with_ellipsis(&item.title, 60),
                    if is_selected && is_active {
                        styles::row_highlight()
                    } else if item.requires_attention {
                        styles::news_attention()
                    } else {
                        Style::default()
                    },
                ),
                // Author
                Span::styled(format!(" - {}", item.author), styles::disabled()),
            ]);

            ListItem::new(line)
        })
        .collect();

    let attention_count = app.news_attention_count();
    let related_count = app.news_related_count();
    let title = if attention_count > 0 || related_count > 0 {
        format!(
            " Arch News ({} attention, {} related) ",
            attention_count, related_count
        )
    } else {
        format!(" Arch News ({}) ", app.news_items.len())
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(if is_active {
                    styles::title_active()
                } else {
                    styles::title_inactive()
                })
                .border_style(if is_active {
                    styles::border_active()
                } else {
                    styles::border_inactive()
                }),
        )
        .highlight_style(styles::row_highlight())
        .highlight_symbol(if is_active { ">> " } else { "   " });

    frame.render_stateful_widget(list, list_area, &mut app.news_list_state);

    // Draw info pane if visible
    if let Some(info_area) = info_area {
        draw_news_info_pane(frame, app.cached_news_info.as_ref(), app.news_scroll, info_area);
    }
}

fn draw_news_info_pane(frame: &mut Frame, info: Option<&NewsInfo>, scroll: u16, area: Rect) {
    let content = if let Some(info) = info {
        let mut lines = vec![
            // Line 1: Title (bold)
            Line::from(Span::styled(&info.title, styles::title_active())),
            // Line 2: Author and date
            Line::from(vec![
                Span::styled("By: ", styles::disabled()),
                Span::styled(&info.author, styles::status_active()),
                Span::styled(" | ", styles::disabled()),
                Span::styled(&info.date, styles::disabled()),
            ]),
            // Line 3: Link
            Line::from(vec![
                Span::styled("Link: ", styles::disabled()),
                Span::styled(&info.link, styles::status_active()),
            ]),
        ];

        // Line 4: Related packages (if any)
        if !info.related_packages.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Related: ", styles::disabled()),
                Span::styled(info.related_packages.join(", "), styles::news_related()),
            ]));
        }

        // Empty separator
        lines.push(Line::from(""));

        // Add all content lines (description)
        for line in &info.content {
            lines.push(Line::from(Span::raw(line.as_str())));
        }

        lines
    } else {
        vec![Line::from(Span::styled(
            "Select a news item to view details",
            styles::disabled(),
        ))]
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Article (Shift+↑/↓ to scroll) ")
                .title_style(styles::title_inactive())
                .border_style(styles::border_inactive()),
        )
        .wrap(ratatui::widgets::Wrap { trim: true })
        .scroll((scroll, 0));

    frame.render_widget(paragraph, area);
}

fn draw_help(frame: &mut Frame, app: &App, area: Rect) {
    let (line1, line2) = match app.tab {
        Tab::Updates => (
            Line::from(vec![
                Span::styled("f/F", styles::help_key()),
                Span::styled(" Filter", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("u", styles::help_key()),
                Span::styled(" Update", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("Enter", styles::help_key()),
                Span::styled(" Update All", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("a/n", styles::help_key()),
                Span::styled(" All/None", styles::help()),
            ]),
            Line::from(vec![
                Span::styled("Space", styles::help_key()),
                Span::styled(" Select", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("?", styles::help_key()),
                Span::styled(" Info", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("r", styles::help_key()),
                Span::styled(" Refresh", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("q", styles::help_key()),
                Span::styled(" Quit", styles::help()),
            ]),
        ),
        Tab::Installed => (
            Line::from(vec![
                Span::styled("f/F", styles::help_key()),
                Span::styled(" Filter", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("d/D", styles::help_key()),
                Span::styled(" Remove/+Deps", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("i/I", styles::help_key()),
                Span::styled(" Reinstall/src", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("a/n", styles::help_key()),
                Span::styled(" All/None", styles::help()),
            ]),
            Line::from(vec![
                Span::styled("Space", styles::help_key()),
                Span::styled(" Select", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("?", styles::help_key()),
                Span::styled(" Info", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("r", styles::help_key()),
                Span::styled(" Refresh", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("q", styles::help_key()),
                Span::styled(" Quit", styles::help()),
            ]),
        ),
        Tab::Orphans => (
            Line::from(vec![
                Span::styled("d/D", styles::help_key()),
                Span::styled(" Remove/+Deps", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("a/n", styles::help_key()),
                Span::styled(" All/None", styles::help()),
            ]),
            Line::from(vec![
                Span::styled("Space", styles::help_key()),
                Span::styled(" Select", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("?", styles::help_key()),
                Span::styled(" Info", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("r", styles::help_key()),
                Span::styled(" Refresh", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("q", styles::help_key()),
                Span::styled(" Quit", styles::help()),
            ]),
        ),
        Tab::Rebuilds => (
            Line::from(vec![
                Span::styled("Enter", styles::help_key()),
                Span::styled(" Fix", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("a/n", styles::help_key()),
                Span::styled(" All/None", styles::help()),
            ]),
            Line::from(vec![
                Span::styled("Space", styles::help_key()),
                Span::styled(" Select", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("?", styles::help_key()),
                Span::styled(" Info", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("r", styles::help_key()),
                Span::styled(" Refresh", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("q", styles::help_key()),
                Span::styled(" Quit", styles::help()),
            ]),
        ),
        Tab::Search => (
            Line::from(vec![
                Span::styled("Type", styles::help_key()),
                Span::styled(" to search", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("Enter", styles::help_key()),
                Span::styled(" Install", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("Esc", styles::help_key()),
                Span::styled(" Clear", styles::help()),
            ]),
            Line::from(vec![
                Span::styled("Space", styles::help_key()),
                Span::styled(" Select", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("?", styles::help_key()),
                Span::styled(" Info", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("q", styles::help_key()),
                Span::styled(" Quit", styles::help()),
            ]),
        ),
        Tab::News => (
            Line::from(vec![
                Span::styled("↑/↓", styles::help_key()),
                Span::styled(" Navigate", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("Shift+↑/↓", styles::help_key()),
                Span::styled(" Scroll", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("*", styles::news_related()),
                Span::styled(" related", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("!", styles::news_attention()),
                Span::styled(" attention", styles::help()),
            ]),
            Line::from(vec![
                Span::styled("?", styles::help_key()),
                Span::styled(" Article", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("r", styles::help_key()),
                Span::styled(" Refresh", styles::help()),
                Span::styled(" | ", styles::help()),
                Span::styled("q", styles::help_key()),
                Span::styled(" Quit", styles::help()),
            ]),
        ),
    };

    let help = Paragraph::new(vec![line1, line2]).alignment(Alignment::Center);

    frame.render_widget(help, area);
}
