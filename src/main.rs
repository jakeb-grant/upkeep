mod action;
mod app;
mod config;
mod rebuilds;
mod ui;
mod updates;

use action::Action;
use anyhow::Result;
use app::App;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::DefaultTerminal;
use std::time::Duration;

fn main() -> Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> Result<()> {
    let mut app = App::new();

    // Initial update check
    app.refresh();

    loop {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.handle_key(key.code) {
                        Action::Quit => break,
                        Action::RunUpdate(packages) => {
                            run_update(terminal, &app, packages)?;
                            app.refresh();
                        }
                        Action::RunRebuild(command) => {
                            run_command(terminal, &command)?;
                            app.refresh_rebuilds();
                        }
                        Action::Uninstall(packages) => {
                            run_uninstall(terminal, &app, packages, false)?;
                            app.refresh_installed();
                            app.refresh_orphans();
                        }
                        Action::UninstallWithDeps(packages) => {
                            run_uninstall(terminal, &app, packages, true)?;
                            app.refresh_installed();
                            app.refresh_orphans();
                        }
                        Action::Reinstall(packages) => {
                            run_reinstall(terminal, &app, packages, false)?;
                            app.refresh_installed();
                        }
                        Action::ForceRebuild(packages) => {
                            run_reinstall(terminal, &app, packages, true)?;
                            app.refresh_installed();
                        }
                        Action::Install(packages) => {
                            run_install(terminal, &app, packages)?;
                            app.refresh_installed();
                            // Re-run search to update installed status
                            app.do_search();
                        }
                        Action::None => {}
                    }
                }
            }
        }

        // Poll for async task completions
        app.poll_tasks();

        // Check if debounce timers expired
        app.check_search_debounce();
        app.check_info_debounce();
    }

    Ok(())
}

fn run_update(terminal: &mut DefaultTerminal, app: &App, packages: Vec<String>) -> Result<()> {
    // Restore terminal to normal mode
    ratatui::restore();

    // Build and run the update command
    let helper = &app.config.aur_helper;
    let status = if packages.is_empty() {
        // Update all
        std::process::Command::new(helper).arg("-Syu").status()?
    } else {
        // Update selected packages
        std::process::Command::new(helper)
            .arg("-S")
            .arg("--needed")
            .args(&packages)
            .status()?
    };

    if !status.success() {
        eprintln!("\nUpdate command exited with status: {}", status);
    }
    eprintln!("\nPress Enter to continue...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    // Re-initialize terminal
    *terminal = ratatui::init();
    Ok(())
}

fn run_command(terminal: &mut DefaultTerminal, command: &str) -> Result<()> {
    ratatui::restore();

    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .status()?;

    if !status.success() {
        eprintln!("\nCommand exited with status: {}", status);
    }
    eprintln!("\nPress Enter to continue...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    *terminal = ratatui::init();
    Ok(())
}

fn run_uninstall(
    terminal: &mut DefaultTerminal,
    app: &App,
    packages: Vec<String>,
    with_deps: bool,
) -> Result<()> {
    ratatui::restore();

    let helper = &app.config.aur_helper;
    let status = if with_deps {
        // Remove with dependencies and config files
        std::process::Command::new(helper)
            .arg("-Rns")
            .args(&packages)
            .status()?
    } else {
        // Simple remove
        std::process::Command::new(helper)
            .arg("-R")
            .args(&packages)
            .status()?
    };

    if !status.success() {
        eprintln!("\nUninstall command exited with status: {}", status);
    }
    eprintln!("\nPress Enter to continue...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    *terminal = ratatui::init();
    Ok(())
}

fn run_reinstall(
    terminal: &mut DefaultTerminal,
    app: &App,
    packages: Vec<String>,
    force_rebuild: bool,
) -> Result<()> {
    ratatui::restore();

    let helper = &app.config.aur_helper;
    let status = if force_rebuild {
        // Force rebuild from source
        std::process::Command::new(helper)
            .arg("-S")
            .arg("--rebuild")
            .args(&packages)
            .status()?
    } else {
        // Reinstall (redownload)
        std::process::Command::new(helper)
            .arg("-S")
            .args(&packages)
            .status()?
    };

    if !status.success() {
        eprintln!("\nReinstall command exited with status: {}", status);
    }
    eprintln!("\nPress Enter to continue...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    *terminal = ratatui::init();
    Ok(())
}

fn run_install(terminal: &mut DefaultTerminal, app: &App, packages: Vec<String>) -> Result<()> {
    ratatui::restore();

    let helper = &app.config.aur_helper;
    let status = std::process::Command::new(helper)
        .arg("-S")
        .args(&packages)
        .status()?;

    if !status.success() {
        eprintln!("\nInstall command exited with status: {}", status);
    }
    eprintln!("\nPress Enter to continue...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    *terminal = ratatui::init();
    Ok(())
}
