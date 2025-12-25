#[derive(Debug, Clone)]
pub enum Action {
    None,
    Quit,
    RunUpdate(Vec<String>),
    RunRebuild(String),
    Uninstall(Vec<String>),
    UninstallWithDeps(Vec<String>),
    Reinstall(Vec<String>),
    ForceRebuild(Vec<String>),
}
