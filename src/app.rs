use crate::action::Action;
use crate::config::Config;
use crate::rebuilds::{check_rebuilds, load_checks, RebuildCheck, RebuildIssue};
use crate::updates::{
    check_aur_updates, check_pacman_updates, fetch_news, filter_items, find_related_packages,
    get_installed_packages, get_orphan_packages, search_packages, InstalledPackage, NewsInfo,
    NewsItem, Package, PackageInfo, PackageSource, SearchResult,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

/// Debounce duration for search - wait this long after last keystroke before searching
pub const SEARCH_DEBOUNCE_MS: u64 = 350;

/// Debounce duration for info pane - wait this long after navigation before fetching
pub const INFO_DEBOUNCE_MS: u64 = 100;

fn clamp_selection(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
    } else {
        let idx = state.selected().unwrap_or(0).min(len - 1);
        state.select(Some(idx));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Updates,
    Installed,
    Orphans,
    Rebuilds,
    Search,
    News,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadingState {
    Idle,
    Loading,
}

pub struct App {
    pub config: Config,
    pub tab: Tab,
    pub packages: Vec<Package>,
    pub installed_packages: Vec<InstalledPackage>,
    pub orphan_packages: Vec<InstalledPackage>,
    pub rebuild_issues: Vec<RebuildIssue>,
    pub rebuild_checks: Vec<RebuildCheck>,
    pub search_results: Vec<SearchResult>,
    pub search_query: String,
    pub search_loading: bool,
    pending_search: Option<String>,
    search_debounce_until: Option<Instant>,
    current_search_id: u64,
    pub list_state: ListState,
    pub installed_list_state: ListState,
    pub orphans_list_state: ListState,
    pub rebuilds_list_state: ListState,
    pub search_list_state: ListState,
    pub news_list_state: ListState,
    pub news_items: Vec<NewsItem>,
    pub news_loading: bool,
    pub news_error: bool,
    pub cached_news_info: Option<NewsInfo>,
    pub news_scroll: u16,
    pub loading: LoadingState,
    pub filter_mode: bool,
    pub filter_text: String,
    pub show_info_pane: bool,
    pub cached_pkg_info: Option<PackageInfo>,
    pub info_loading: bool,
    pending_info_fetch: Option<(String, Option<PackageInfo>)>, // (name, fallback for AUR)
    info_debounce_until: Option<Instant>,
    current_info_id: u64,
    pending_tasks: usize,
    task_rx: Option<Receiver<TaskResult>>,
    task_tx: Sender<TaskResult>,
}

enum TaskResult {
    Updates(Vec<Package>, Vec<Package>),
    Installed(Vec<InstalledPackage>),
    Orphans(Vec<InstalledPackage>),
    Rebuilds(Vec<RebuildIssue>),
    Search(u64, Vec<SearchResult>),        // (search_id, results)
    PackageInfo(u64, Option<PackageInfo>), // (info_id, info)
    News(Result<Vec<NewsItem>, String>),   // Ok(items) or Err(error_message)
}

impl App {
    pub fn new() -> Self {
        let config = Config::load().unwrap_or_default();
        let rebuild_checks = load_checks().unwrap_or_default();
        let (tx, rx) = mpsc::channel();

        Self {
            config,
            tab: Tab::Updates,
            packages: Vec::new(),
            installed_packages: Vec::new(),
            orphan_packages: Vec::new(),
            rebuild_issues: Vec::new(),
            rebuild_checks,
            search_results: Vec::new(),
            search_query: String::new(),
            search_loading: false,
            pending_search: None,
            search_debounce_until: None,
            current_search_id: 0,
            list_state: ListState::default(),
            installed_list_state: ListState::default(),
            orphans_list_state: ListState::default(),
            rebuilds_list_state: ListState::default(),
            search_list_state: ListState::default(),
            news_list_state: ListState::default(),
            news_items: Vec::new(),
            news_loading: false,
            news_error: false,
            cached_news_info: None,
            news_scroll: 0,
            loading: LoadingState::Idle,
            filter_mode: false,
            filter_text: String::new(),
            show_info_pane: true,
            cached_pkg_info: None,
            info_loading: false,
            pending_info_fetch: None,
            info_debounce_until: None,
            current_info_id: 0,
            pending_tasks: 0,
            task_rx: Some(rx),
            task_tx: tx,
        }
    }

    pub fn refresh(&mut self) {
        self.loading = LoadingState::Loading;
        self.pending_tasks = 3;
        let tx = self.task_tx.clone();
        let checks = self.rebuild_checks.clone();
        let aur_helper = self.config.aur_helper.clone();

        thread::spawn(move || {
            let pacman = check_pacman_updates();
            let aur = check_aur_updates(&aur_helper);
            let _ = tx.send(TaskResult::Updates(pacman, aur));

            let installed = get_installed_packages();
            let _ = tx.send(TaskResult::Installed(installed));

            let issues = check_rebuilds(&checks);
            let _ = tx.send(TaskResult::Rebuilds(issues));
        });
    }

    pub fn refresh_installed(&mut self) {
        self.loading = LoadingState::Loading;
        self.pending_tasks += 1;
        let tx = self.task_tx.clone();

        thread::spawn(move || {
            let installed = get_installed_packages();
            let _ = tx.send(TaskResult::Installed(installed));
        });
    }

    pub fn refresh_rebuilds(&mut self) {
        self.loading = LoadingState::Loading;
        self.pending_tasks += 1;
        let tx = self.task_tx.clone();
        let checks = self.rebuild_checks.clone();

        thread::spawn(move || {
            let issues = check_rebuilds(&checks);
            let _ = tx.send(TaskResult::Rebuilds(issues));
        });
    }

    pub fn refresh_orphans(&mut self) {
        self.loading = LoadingState::Loading;
        self.pending_tasks += 1;
        let tx = self.task_tx.clone();

        thread::spawn(move || {
            let orphans = get_orphan_packages();
            let _ = tx.send(TaskResult::Orphans(orphans));
        });
    }

    pub fn refresh_news(&mut self) {
        self.news_loading = true;
        self.news_error = false;
        let tx = self.task_tx.clone();
        // Get installed package names for matching
        let installed_names: Vec<String> = self
            .installed_packages
            .iter()
            .map(|p| p.name.clone())
            .collect();

        thread::spawn(move || {
            let news = fetch_news(&installed_names);
            let _ = tx.send(TaskResult::News(news));
        });
    }

    pub fn poll_tasks(&mut self) {
        // Collect results first to avoid borrow issues
        let results: Vec<TaskResult> = if let Some(rx) = &self.task_rx {
            let mut collected = Vec::new();
            while let Ok(result) = rx.try_recv() {
                collected.push(result);
            }
            collected
        } else {
            Vec::new()
        };

        // Now process results
        for result in results {
            match result {
                TaskResult::Updates(pacman, aur) => {
                    self.pending_tasks = self.pending_tasks.saturating_sub(1);
                    self.packages = pacman;
                    self.packages.extend(aur);
                    self.clamp_list_selection();
                    if self.show_info_pane && self.tab == Tab::Updates {
                        self.refresh_package_info();
                    }
                }
                TaskResult::Installed(installed) => {
                    self.pending_tasks = self.pending_tasks.saturating_sub(1);
                    self.installed_packages = installed;
                    self.clamp_installed_selection();
                    if self.show_info_pane && self.tab == Tab::Installed {
                        self.refresh_package_info();
                    }
                    // Re-match news items now that we have installed packages
                    self.rematch_news_packages();
                }
                TaskResult::Orphans(orphans) => {
                    self.pending_tasks = self.pending_tasks.saturating_sub(1);
                    self.orphan_packages = orphans;
                    self.clamp_orphans_selection();
                    if self.show_info_pane && self.tab == Tab::Orphans {
                        self.refresh_package_info();
                    }
                }
                TaskResult::Rebuilds(issues) => {
                    self.pending_tasks = self.pending_tasks.saturating_sub(1);
                    self.rebuild_issues = issues;
                    self.clamp_rebuilds_selection();
                    if self.show_info_pane && self.tab == Tab::Rebuilds {
                        self.refresh_package_info();
                    }
                }
                TaskResult::Search(search_id, results) => {
                    // Only use results if this is the current search (ignore stale results)
                    if search_id == self.current_search_id {
                        self.search_results = results;
                        self.search_loading = false;
                        self.clamp_search_selection();
                        if self.search_results.is_empty() {
                            self.search_list_state.select(None);
                        } else if self.search_list_state.selected().is_none() {
                            self.search_list_state.select(Some(0));
                        }
                        if self.show_info_pane {
                            self.refresh_package_info();
                        }
                    }
                    // Stale results are silently discarded
                }
                TaskResult::PackageInfo(info_id, info) => {
                    // Only use results if this is the current info request (ignore stale)
                    if info_id == self.current_info_id {
                        self.cached_pkg_info = info;
                        self.info_loading = false;
                    }
                    // Stale results are silently discarded
                }
                TaskResult::News(result) => {
                    self.news_loading = false;
                    match result {
                        Ok(items) => {
                            self.news_items = items;
                            self.news_error = false;
                            self.clamp_news_selection();
                            // Auto-select first item if none selected
                            if self.news_list_state.selected().is_none()
                                && !self.news_items.is_empty()
                            {
                                self.news_list_state.select(Some(0));
                            }
                            if self.show_info_pane && self.tab == Tab::News {
                                self.refresh_news_info();
                            }
                        }
                        Err(_) => {
                            self.news_error = true;
                        }
                    }
                }
            }
        }

        if self.pending_tasks == 0 {
            self.loading = LoadingState::Idle;
        }
    }

    fn clamp_list_selection(&mut self) {
        clamp_selection(&mut self.list_state, self.packages.len());
    }

    fn clamp_installed_selection(&mut self) {
        clamp_selection(&mut self.installed_list_state, self.installed_packages.len());
    }

    fn clamp_rebuilds_selection(&mut self) {
        clamp_selection(&mut self.rebuilds_list_state, self.rebuild_issues.len());
    }

    fn clamp_orphans_selection(&mut self) {
        clamp_selection(&mut self.orphans_list_state, self.orphan_packages.len());
    }

    fn clamp_news_selection(&mut self) {
        clamp_selection(&mut self.news_list_state, self.news_items.len());
    }

    fn load_tab_data(&mut self) {
        match self.tab {
            Tab::Installed if self.installed_packages.is_empty() => self.refresh_installed(),
            Tab::Orphans if self.orphan_packages.is_empty() => self.refresh_orphans(),
            Tab::News if self.news_items.is_empty() => self.refresh_news(),
            _ => {}
        }
    }

    fn clamp_filter_selection(&mut self) {
        match self.tab {
            Tab::Updates => {
                let len = self.filtered_updates().len();
                clamp_selection(&mut self.list_state, len);
            }
            Tab::Installed => {
                let len = self.filtered_installed().len();
                clamp_selection(&mut self.installed_list_state, len);
            }
            Tab::Orphans | Tab::Rebuilds | Tab::Search | Tab::News => {}
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        // Handle filter mode input
        if self.filter_mode {
            match key.code {
                KeyCode::Esc => {
                    self.filter_mode = false;
                    self.filter_text.clear();
                    Action::None
                }
                KeyCode::Char('F') => {
                    self.filter_mode = false;
                    Action::None
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.move_selection(1);
                    Action::None
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.move_selection(-1);
                    Action::None
                }
                KeyCode::Char(' ') => {
                    self.toggle_selection();
                    Action::None
                }
                KeyCode::Backspace => {
                    self.filter_text.pop();
                    self.clamp_filter_selection();
                    Action::None
                }
                KeyCode::Char(c) => {
                    self.filter_text.push(c);
                    self.clamp_filter_selection();
                    Action::None
                }
                _ => Action::None,
            }
        } else if self.tab == Tab::Search {
            self.handle_search_key(key.code)
        } else if self.tab == Tab::News {
            self.handle_news_key(key)
        } else {
            self.handle_normal_key(key.code)
        }
    }

    fn handle_search_key(&mut self, key: KeyCode) -> Action {
        match key {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Esc => {
                if !self.search_query.is_empty() {
                    self.search_query.clear();
                    self.search_results.clear();
                    self.search_list_state.select(None);
                    Action::None
                } else {
                    Action::Quit
                }
            }
            KeyCode::Tab => {
                self.tab = Tab::News;
                self.load_tab_data();
                if self.show_info_pane {
                    self.refresh_news_info();
                }
                Action::None
            }
            KeyCode::BackTab => {
                self.tab = Tab::Rebuilds;
                self.load_tab_data();
                if self.show_info_pane {
                    self.refresh_package_info();
                }
                Action::None
            }
            KeyCode::Down => {
                self.move_selection(1);
                Action::None
            }
            KeyCode::Up => {
                self.move_selection(-1);
                Action::None
            }
            KeyCode::Char(' ') => {
                self.toggle_selection();
                Action::None
            }
            KeyCode::Char('?') => {
                self.show_info_pane = !self.show_info_pane;
                if self.show_info_pane {
                    self.refresh_package_info();
                } else {
                    self.cached_pkg_info = None;
                    self.pending_info_fetch = None;
                    self.info_debounce_until = None;
                    self.info_loading = false;
                    self.current_info_id += 1; // Invalidate in-flight fetches
                }
                Action::None
            }
            KeyCode::Enter => self.install_selected(),
            KeyCode::Backspace => {
                self.search_query.pop();
                self.do_search();
                Action::None
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.do_search();
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_news_key(&mut self, key: KeyEvent) -> Action {
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
            KeyCode::Tab => {
                self.tab = Tab::Updates;
                self.load_tab_data();
                if self.show_info_pane {
                    self.refresh_package_info();
                }
                Action::None
            }
            KeyCode::BackTab => {
                self.tab = Tab::Search;
                self.load_tab_data();
                if self.show_info_pane {
                    self.refresh_package_info();
                }
                Action::None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if shift {
                    // Shift+Down: scroll article
                    self.news_scroll = self.news_scroll.saturating_add(3);
                    self.clamp_news_scroll();
                } else {
                    // Down/j: navigate list
                    self.move_news_selection(1);
                }
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if shift {
                    // Shift+Up: scroll article
                    self.news_scroll = self.news_scroll.saturating_sub(3);
                } else {
                    // Up/k: navigate list
                    self.move_news_selection(-1);
                }
                Action::None
            }
            KeyCode::PageDown => {
                self.news_scroll = self.news_scroll.saturating_add(10);
                self.clamp_news_scroll();
                Action::None
            }
            KeyCode::PageUp => {
                self.news_scroll = self.news_scroll.saturating_sub(10);
                Action::None
            }
            KeyCode::Char('r') => {
                self.refresh_news();
                Action::None
            }
            KeyCode::Char('?') => {
                self.show_info_pane = !self.show_info_pane;
                if self.show_info_pane {
                    self.refresh_news_info();
                } else {
                    self.cached_news_info = None;
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_normal_key(&mut self, key: KeyCode) -> Action {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
            KeyCode::Tab => {
                self.tab = match self.tab {
                    Tab::Updates => Tab::Installed,
                    Tab::Installed => Tab::Orphans,
                    Tab::Orphans => Tab::Rebuilds,
                    Tab::Rebuilds => Tab::Search,
                    Tab::Search => Tab::News,
                    Tab::News => Tab::Updates,
                };
                self.filter_mode = false;
                self.filter_text.clear();
                self.load_tab_data();
                if self.show_info_pane {
                    self.refresh_package_info();
                }
                Action::None
            }
            KeyCode::BackTab => {
                self.tab = match self.tab {
                    Tab::Updates => Tab::News,
                    Tab::Installed => Tab::Updates,
                    Tab::Orphans => Tab::Installed,
                    Tab::Rebuilds => Tab::Orphans,
                    Tab::Search => Tab::Rebuilds,
                    Tab::News => Tab::Search,
                };
                self.filter_mode = false;
                self.filter_text.clear();
                self.load_tab_data();
                if self.show_info_pane {
                    self.refresh_package_info();
                }
                Action::None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_selection(1);
                Action::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_selection(-1);
                Action::None
            }
            KeyCode::Char(' ') => {
                self.toggle_selection();
                Action::None
            }
            KeyCode::Char('a') => {
                self.select_all();
                Action::None
            }
            KeyCode::Char('n') => {
                self.select_none();
                Action::None
            }
            KeyCode::Char('r') => {
                match self.tab {
                    Tab::Updates => self.refresh(),
                    Tab::Installed => self.refresh_installed(),
                    Tab::Orphans => self.refresh_orphans(),
                    Tab::Rebuilds => self.refresh_rebuilds(),
                    Tab::Search | Tab::News => {} // Search has its own refresh, News handled by handle_news_key
                }
                Action::None
            }
            KeyCode::Char('u') => self.run_selected_update(),
            KeyCode::Char('d') => self.uninstall_selected(false),
            KeyCode::Char('D') => self.uninstall_selected(true),
            KeyCode::Char('i') => self.reinstall_selected(false),
            KeyCode::Char('I') => self.reinstall_selected(true),
            KeyCode::Char('f') => {
                if self.tab == Tab::Updates || self.tab == Tab::Installed {
                    self.filter_mode = true;
                }
                Action::None
            }
            KeyCode::Enter => self.run_action(),
            KeyCode::Char('?') => {
                self.show_info_pane = !self.show_info_pane;
                if self.show_info_pane {
                    self.refresh_package_info();
                } else {
                    self.cached_pkg_info = None;
                    self.pending_info_fetch = None;
                    self.info_debounce_until = None;
                    self.info_loading = false;
                    self.current_info_id += 1; // Invalidate in-flight fetches
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn move_selection(&mut self, delta: i32) {
        match self.tab {
            Tab::Updates => {
                let filtered = self.filtered_updates();
                if filtered.is_empty() {
                    return;
                }
                let current = self.list_state.selected().unwrap_or(0) as i32;
                let new = (current + delta).clamp(0, filtered.len() as i32 - 1) as usize;
                self.list_state.select(Some(new));
            }
            Tab::Installed => {
                let filtered = self.filtered_installed();
                if filtered.is_empty() {
                    return;
                }
                let current = self.installed_list_state.selected().unwrap_or(0) as i32;
                let new = (current + delta).clamp(0, filtered.len() as i32 - 1) as usize;
                self.installed_list_state.select(Some(new));
            }
            Tab::Orphans => {
                if self.orphan_packages.is_empty() {
                    return;
                }
                let current = self.orphans_list_state.selected().unwrap_or(0) as i32;
                let new =
                    (current + delta).clamp(0, self.orphan_packages.len() as i32 - 1) as usize;
                self.orphans_list_state.select(Some(new));
            }
            Tab::Rebuilds => {
                if self.rebuild_issues.is_empty() {
                    return;
                }
                let current = self.rebuilds_list_state.selected().unwrap_or(0) as i32;
                let new =
                    (current + delta).clamp(0, self.rebuild_issues.len() as i32 - 1) as usize;
                self.rebuilds_list_state.select(Some(new));
            }
            Tab::Search => {
                if self.search_results.is_empty() {
                    return;
                }
                let current = self.search_list_state.selected().unwrap_or(0) as i32;
                let new =
                    (current + delta).clamp(0, self.search_results.len() as i32 - 1) as usize;
                self.search_list_state.select(Some(new));
            }
            Tab::News => {
                // News uses move_news_selection instead
                return;
            }
        }

        // Refresh package info if the info pane is visible
        if self.show_info_pane {
            self.refresh_package_info();
        }
    }

    fn toggle_selection(&mut self) {
        match self.tab {
            Tab::Updates => {
                if let Some(filter_idx) = self.list_state.selected() {
                    let real_idx = self.filtered_updates().get(filter_idx).map(|(idx, _)| *idx);
                    if let Some(real_idx) = real_idx {
                        if let Some(pkg) = self.packages.get_mut(real_idx) {
                            pkg.selected = !pkg.selected;
                        }
                    }
                }
            }
            Tab::Installed => {
                if let Some(filter_idx) = self.installed_list_state.selected() {
                    // Get real index first to avoid borrow conflict
                    let real_idx = self.filtered_installed().get(filter_idx).map(|(idx, _)| *idx);
                    if let Some(real_idx) = real_idx {
                        if let Some(pkg) = self.installed_packages.get_mut(real_idx) {
                            pkg.selected = !pkg.selected;
                        }
                    }
                }
            }
            Tab::Orphans => {
                if let Some(i) = self.orphans_list_state.selected() {
                    if let Some(pkg) = self.orphan_packages.get_mut(i) {
                        pkg.selected = !pkg.selected;
                    }
                }
            }
            Tab::Rebuilds => {
                if let Some(i) = self.rebuilds_list_state.selected() {
                    if let Some(issue) = self.rebuild_issues.get_mut(i) {
                        issue.selected = !issue.selected;
                    }
                }
            }
            Tab::Search => {
                if let Some(i) = self.search_list_state.selected() {
                    if let Some(result) = self.search_results.get_mut(i) {
                        result.selected = !result.selected;
                    }
                }
            }
            Tab::News => {} // News items are not selectable
        }
    }

    fn select_all(&mut self) {
        match self.tab {
            Tab::Updates => {
                let indices: Vec<usize> = self.filtered_updates().iter().map(|(i, _)| *i).collect();
                for idx in indices {
                    if let Some(pkg) = self.packages.get_mut(idx) {
                        pkg.selected = true;
                    }
                }
            }
            Tab::Installed => {
                // Only select filtered packages
                let indices: Vec<usize> = self.filtered_installed().iter().map(|(i, _)| *i).collect();
                for idx in indices {
                    if let Some(pkg) = self.installed_packages.get_mut(idx) {
                        pkg.selected = true;
                    }
                }
            }
            Tab::Orphans => {
                for pkg in &mut self.orphan_packages {
                    pkg.selected = true;
                }
            }
            Tab::Rebuilds => {
                for issue in &mut self.rebuild_issues {
                    issue.selected = true;
                }
            }
            Tab::Search => {
                for result in &mut self.search_results {
                    if !result.installed {
                        result.selected = true;
                    }
                }
            }
            Tab::News => {} // News items are not selectable
        }
    }

    fn select_none(&mut self) {
        match self.tab {
            Tab::Updates => {
                let indices: Vec<usize> = self.filtered_updates().iter().map(|(i, _)| *i).collect();
                for idx in indices {
                    if let Some(pkg) = self.packages.get_mut(idx) {
                        pkg.selected = false;
                    }
                }
            }
            Tab::Installed => {
                // Only deselect filtered packages
                let indices: Vec<usize> = self.filtered_installed().iter().map(|(i, _)| *i).collect();
                for idx in indices {
                    if let Some(pkg) = self.installed_packages.get_mut(idx) {
                        pkg.selected = false;
                    }
                }
            }
            Tab::Orphans => {
                for pkg in &mut self.orphan_packages {
                    pkg.selected = false;
                }
            }
            Tab::Rebuilds => {
                for issue in &mut self.rebuild_issues {
                    issue.selected = false;
                }
            }
            Tab::Search => {
                for result in &mut self.search_results {
                    result.selected = false;
                }
            }
            Tab::News => {} // News items are not selectable
        }
    }

    fn run_selected_update(&self) -> Action {
        if self.tab != Tab::Updates {
            return Action::None;
        }

        let selected: Vec<String> = self
            .packages
            .iter()
            .filter(|p| p.selected)
            .map(|p| p.name.clone())
            .collect();

        if selected.is_empty() {
            return Action::None;
        }

        Action::RunUpdate(selected)
    }

    fn uninstall_selected(&self, with_deps: bool) -> Action {
        let packages = match self.tab {
            Tab::Installed => &self.installed_packages,
            Tab::Orphans => &self.orphan_packages,
            _ => return Action::None,
        };

        let selected: Vec<String> = packages
            .iter()
            .filter(|p| p.selected)
            .map(|p| p.name.clone())
            .collect();

        if selected.is_empty() {
            // Use current selection if nothing explicitly selected
            let pkg_name = match self.tab {
                Tab::Installed => {
                    // Installed tab has filter - translate filter index to real index
                    if let Some(filter_idx) = self.installed_list_state.selected() {
                        let filtered = self.filtered_installed();
                        if let Some((real_idx, _)) = filtered.get(filter_idx) {
                            if let Some(pkg) = self.installed_packages.get(*real_idx) {
                                pkg.name.clone()
                            } else {
                                return Action::None;
                            }
                        } else {
                            return Action::None;
                        }
                    } else {
                        return Action::None;
                    }
                }
                Tab::Orphans => {
                    // Orphans has no filter - use index directly
                    if let Some(idx) = self.orphans_list_state.selected() {
                        if let Some(pkg) = self.orphan_packages.get(idx) {
                            pkg.name.clone()
                        } else {
                            return Action::None;
                        }
                    } else {
                        return Action::None;
                    }
                }
                _ => return Action::None,
            };

            return if with_deps {
                Action::UninstallWithDeps(vec![pkg_name])
            } else {
                Action::Uninstall(vec![pkg_name])
            };
        }

        if with_deps {
            Action::UninstallWithDeps(selected)
        } else {
            Action::Uninstall(selected)
        }
    }

    fn reinstall_selected(&self, force_rebuild: bool) -> Action {
        if self.tab != Tab::Installed {
            return Action::None;
        }

        let selected: Vec<String> = self
            .installed_packages
            .iter()
            .filter(|p| p.selected)
            .map(|p| p.name.clone())
            .collect();

        if selected.is_empty() {
            // Use current selection if nothing explicitly selected
            if let Some(filter_idx) = self.installed_list_state.selected() {
                let filtered = self.filtered_installed();
                if let Some((real_idx, _)) = filtered.get(filter_idx) {
                    if let Some(pkg) = self.installed_packages.get(*real_idx) {
                        return if force_rebuild {
                            Action::ForceRebuild(vec![pkg.name.clone()])
                        } else {
                            Action::Reinstall(vec![pkg.name.clone()])
                        };
                    }
                }
            }
            return Action::None;
        }

        if force_rebuild {
            Action::ForceRebuild(selected)
        } else {
            Action::Reinstall(selected)
        }
    }

    fn run_action(&self) -> Action {
        match self.tab {
            Tab::Updates => {
                // Enter = update all
                Action::RunUpdate(Vec::new())
            }
            Tab::Installed | Tab::Orphans => {
                // Enter does nothing on installed/orphans tab - use specific keys
                Action::None
            }
            Tab::Rebuilds => {
                // Run selected rebuild or current one
                let selected: Vec<&RebuildIssue> =
                    self.rebuild_issues.iter().filter(|i| i.selected).collect();

                if !selected.is_empty() {
                    let commands: Vec<String> =
                        selected.iter().map(|i| i.rebuild_command.clone()).collect();
                    Action::RunRebuild(commands.join(" && "))
                } else if let Some(i) = self.rebuilds_list_state.selected() {
                    if let Some(issue) = self.rebuild_issues.get(i) {
                        Action::RunRebuild(issue.rebuild_command.clone())
                    } else {
                        Action::None
                    }
                } else {
                    Action::None
                }
            }
            Tab::Search | Tab::News => {
                // Enter = install selected (handled by handle_search_key)
                // News has no action on Enter
                Action::None
            }
        }
    }

    pub fn pacman_count(&self) -> usize {
        self.packages
            .iter()
            .filter(|p| p.source == PackageSource::Pacman)
            .count()
    }

    pub fn aur_count(&self) -> usize {
        self.packages
            .iter()
            .filter(|p| p.source == PackageSource::Aur)
            .count()
    }

    pub fn installed_count(&self) -> usize {
        self.installed_packages.len()
    }

    pub fn installed_aur_count(&self) -> usize {
        self.installed_packages
            .iter()
            .filter(|p| p.source == PackageSource::Aur)
            .count()
    }

    pub fn orphan_count(&self) -> usize {
        self.orphan_packages.len()
    }

    pub fn filtered_installed(&self) -> Vec<(usize, &InstalledPackage)> {
        filter_items(&self.installed_packages, &self.filter_text)
    }

    pub fn filtered_updates(&self) -> Vec<(usize, &Package)> {
        filter_items(&self.packages, &self.filter_text)
    }

    fn refresh_package_info(&mut self) {
        // For Search tab, prepare fallback from SearchResult (for uninstalled AUR packages)
        if self.tab == Tab::Search {
            if let Some(idx) = self.search_list_state.selected() {
                if let Some(result) = self.search_results.get(idx) {
                    let fallback = PackageInfo {
                        name: result.name.clone(),
                        version: result.version.clone(),
                        description: result.description.clone(),
                        size: String::new(),
                        repository: result.repository.clone(),
                        install_date: None,
                        install_reason: None,
                        url: None,
                        build_date: None,
                        maintainer: None,
                        votes: None,
                        required_by: Vec::new(),
                        optional_for: Vec::new(),
                    };
                    self.pending_info_fetch = Some((result.name.clone(), Some(fallback)));
                    self.info_debounce_until =
                        Some(Instant::now() + Duration::from_millis(INFO_DEBOUNCE_MS));
                    return;
                }
            }
            // No selection - clear pending and info
            self.pending_info_fetch = None;
            self.info_debounce_until = None;
            self.cached_pkg_info = None;
            self.info_loading = false;
            return;
        }

        // For other tabs, set pending fetch (no fallback needed)
        if let Some(pkg_name) = self.get_selected_package_name() {
            self.pending_info_fetch = Some((pkg_name, None));
            self.info_debounce_until =
                Some(Instant::now() + Duration::from_millis(INFO_DEBOUNCE_MS));
        } else {
            self.pending_info_fetch = None;
            self.info_debounce_until = None;
            self.cached_pkg_info = None;
            self.info_loading = false;
        }
    }

    fn get_selected_package_name(&self) -> Option<String> {
        match self.tab {
            Tab::Updates => {
                let filter_idx = self.list_state.selected()?;
                let filtered = self.filtered_updates();
                let (real_idx, _) = filtered.get(filter_idx)?;
                self.packages.get(*real_idx).map(|p| p.name.clone())
            }
            Tab::Installed => {
                let filter_idx = self.installed_list_state.selected()?;
                let filtered = self.filtered_installed();
                let (real_idx, _) = filtered.get(filter_idx)?;
                self.installed_packages.get(*real_idx).map(|p| p.name.clone())
            }
            Tab::Orphans => {
                let idx = self.orphans_list_state.selected()?;
                self.orphan_packages.get(idx).map(|p| p.name.clone())
            }
            Tab::Rebuilds => {
                let idx = self.rebuilds_list_state.selected()?;
                self.rebuild_issues.get(idx).map(|i| i.name.clone())
            }
            Tab::Search => {
                let idx = self.search_list_state.selected()?;
                self.search_results.get(idx).map(|r| r.name.clone())
            }
            Tab::News => None, // News items are not packages
        }
    }

    fn clamp_search_selection(&mut self) {
        clamp_selection(&mut self.search_list_state, self.search_results.len());
    }

    /// Called on each keystroke - sets up debounced search
    pub fn do_search(&mut self) {
        if self.search_query.len() >= 2 {
            // Set pending search with debounce
            self.pending_search = Some(self.search_query.clone());
            self.search_debounce_until =
                Some(Instant::now() + Duration::from_millis(SEARCH_DEBOUNCE_MS));
        } else {
            // Query too short - clear results immediately
            self.pending_search = None;
            self.search_debounce_until = None;
            self.search_results.clear();
            self.search_list_state.select(None);
            self.search_loading = false;
            // Invalidate any in-flight searches
            self.current_search_id += 1;
        }
    }

    /// Check if debounce timer expired and trigger search if so
    /// Returns true if a search was triggered
    pub fn check_search_debounce(&mut self) -> bool {
        if let (Some(query), Some(until)) = (&self.pending_search, self.search_debounce_until) {
            if Instant::now() >= until {
                let query = query.clone();
                self.pending_search = None;
                self.search_debounce_until = None;
                self.trigger_search(&query);
                return true;
            }
        }
        false
    }

    /// Spawn background search thread
    fn trigger_search(&mut self, query: &str) {
        self.current_search_id += 1;
        self.search_loading = true;

        let search_id = self.current_search_id;
        let query = query.to_string();
        let tx = self.task_tx.clone();

        thread::spawn(move || {
            let results = search_packages(&query);
            let _ = tx.send(TaskResult::Search(search_id, results));
        });
    }

    /// Check if info debounce timer expired and trigger fetch if so
    pub fn check_info_debounce(&mut self) -> bool {
        if let (Some((name, fallback)), Some(until)) =
            (&self.pending_info_fetch, self.info_debounce_until)
        {
            if Instant::now() >= until {
                let name = name.clone();
                let fallback = fallback.clone();
                self.pending_info_fetch = None;
                self.info_debounce_until = None;
                self.trigger_info_fetch(&name, fallback);
                return true;
            }
        }
        false
    }

    /// Spawn background info fetch thread
    fn trigger_info_fetch(&mut self, name: &str, fallback: Option<PackageInfo>) {
        self.current_info_id += 1;
        self.info_loading = true;

        let info_id = self.current_info_id;
        let name = name.to_string();
        let tx = self.task_tx.clone();

        thread::spawn(move || {
            // Try pacman first, fall back to provided fallback (for uninstalled AUR packages)
            let info = PackageInfo::fetch(&name).or(fallback);
            let _ = tx.send(TaskResult::PackageInfo(info_id, info));
        });
    }

    fn move_news_selection(&mut self, delta: i32) {
        if self.news_items.is_empty() {
            return;
        }
        let current = self.news_list_state.selected().unwrap_or(0) as i32;
        let new = (current + delta).clamp(0, self.news_items.len() as i32 - 1) as usize;
        self.news_list_state.select(Some(new));
        self.news_scroll = 0; // Reset scroll when changing selection

        if self.show_info_pane {
            self.refresh_news_info();
        }
    }

    /// Clamp news scroll to prevent scrolling past content
    fn clamp_news_scroll(&mut self) {
        if let Some(info) = &self.cached_news_info {
            // Calculate approximate max scroll based on content lines
            // Header: 4-5 lines (title, author/date, link, related, empty line)
            // Content: info.content.len() lines
            let header_lines = if info.related_packages.is_empty() { 4 } else { 5 };
            let total_lines = header_lines + info.content.len();
            // Allow scrolling until only a few lines remain visible
            let max_scroll = total_lines.saturating_sub(3) as u16;
            self.news_scroll = self.news_scroll.min(max_scroll);
        }
    }

    fn refresh_news_info(&mut self) {
        if let Some(idx) = self.news_list_state.selected() {
            if let Some(item) = self.news_items.get(idx) {
                self.cached_news_info = Some(item.to_info());
            } else {
                self.cached_news_info = None;
            }
        } else {
            self.cached_news_info = None;
        }
    }

    /// Re-match news items against installed packages
    /// Called when installed packages list is updated to fix race condition
    fn rematch_news_packages(&mut self) {
        if self.news_items.is_empty() || self.installed_packages.is_empty() {
            return;
        }

        let installed_names: Vec<String> = self
            .installed_packages
            .iter()
            .map(|p| p.name.clone())
            .collect();

        for item in &mut self.news_items {
            let full_text = format!("{} {}", item.title, item.description);
            item.related_packages = find_related_packages(&full_text, &installed_names);
        }

        // Refresh info pane if on news tab
        if self.show_info_pane && self.tab == Tab::News {
            self.refresh_news_info();
        }
    }

    pub fn news_attention_count(&self) -> usize {
        self.news_items.iter().filter(|n| n.requires_attention).count()
    }

    pub fn news_related_count(&self) -> usize {
        self.news_items.iter().filter(|n| !n.related_packages.is_empty()).count()
    }

    pub fn install_selected(&self) -> Action {
        if self.tab != Tab::Search {
            return Action::None;
        }

        let selected: Vec<String> = self
            .search_results
            .iter()
            .filter(|r| r.selected && !r.installed)
            .map(|r| r.name.clone())
            .collect();

        if selected.is_empty() {
            // Use current selection if nothing explicitly selected
            if let Some(idx) = self.search_list_state.selected() {
                if let Some(result) = self.search_results.get(idx) {
                    if !result.installed {
                        return Action::Install(vec![result.name.clone()]);
                    }
                }
            }
            return Action::None;
        }

        Action::Install(selected)
    }
}
