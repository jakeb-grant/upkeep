use crate::action::Action;
use crate::config::Config;
use crate::rebuilds::{check_rebuilds, load_checks, RebuildCheck, RebuildIssue};
use crate::updates::{
    check_aur_updates, check_pacman_updates, filter_items, get_installed_packages,
    get_orphan_packages, InstalledPackage, Package, PackageSource,
};
use crossterm::event::KeyCode;
use ratatui::widgets::ListState;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

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
    pub list_state: ListState,
    pub installed_list_state: ListState,
    pub orphans_list_state: ListState,
    pub rebuilds_list_state: ListState,
    pub loading: LoadingState,
    pub filter_mode: bool,
    pub filter_text: String,
    pending_tasks: usize,
    task_rx: Option<Receiver<TaskResult>>,
    task_tx: Sender<TaskResult>,
}

enum TaskResult {
    Updates(Vec<Package>, Vec<Package>),
    Installed(Vec<InstalledPackage>),
    Orphans(Vec<InstalledPackage>),
    Rebuilds(Vec<RebuildIssue>),
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
            list_state: ListState::default(),
            installed_list_state: ListState::default(),
            orphans_list_state: ListState::default(),
            rebuilds_list_state: ListState::default(),
            loading: LoadingState::Idle,
            filter_mode: false,
            filter_text: String::new(),
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
            self.pending_tasks = self.pending_tasks.saturating_sub(1);
            match result {
                TaskResult::Updates(pacman, aur) => {
                    self.packages = pacman;
                    self.packages.extend(aur);
                    self.clamp_list_selection();
                }
                TaskResult::Installed(installed) => {
                    self.installed_packages = installed;
                    self.clamp_installed_selection();
                }
                TaskResult::Orphans(orphans) => {
                    self.orphan_packages = orphans;
                    self.clamp_orphans_selection();
                }
                TaskResult::Rebuilds(issues) => {
                    self.rebuild_issues = issues;
                    self.clamp_rebuilds_selection();
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

    fn load_tab_data(&mut self) {
        match self.tab {
            Tab::Installed if self.installed_packages.is_empty() => self.refresh_installed(),
            Tab::Orphans if self.orphan_packages.is_empty() => self.refresh_orphans(),
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
            Tab::Orphans | Tab::Rebuilds => {}
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Action {
        // Handle filter mode input
        if self.filter_mode {
            match key {
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
        } else {
            self.handle_normal_key(key)
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
                    Tab::Rebuilds => Tab::Updates,
                };
                self.filter_mode = false;
                self.filter_text.clear();
                self.load_tab_data();
                Action::None
            }
            KeyCode::BackTab => {
                self.tab = match self.tab {
                    Tab::Updates => Tab::Rebuilds,
                    Tab::Installed => Tab::Updates,
                    Tab::Orphans => Tab::Installed,
                    Tab::Rebuilds => Tab::Orphans,
                };
                self.filter_mode = false;
                self.filter_text.clear();
                self.load_tab_data();
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
}
