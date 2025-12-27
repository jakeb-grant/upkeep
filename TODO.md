# Upkeep Feature Roadmap

## Phase 1: Quick Wins ✓

- [x] **1. Filter on Updates tab** - Consistency with Installed tab, reuse existing code
- [x] **2. Orphan packages tab** - Packages no longer needed as deps (`pacman -Qdt`), easy cleanup

## Phase 2: Core Features ✓

- [x] **3. Package search tab** - Search across repos + AUR for new packages to install
  - Live search with 350ms debounce
  - `pacman -Ss` for official repos + AUR RPC for AUR packages
  - Async/non-blocking search
  - Install with Enter

- [x] **4. Package info pane** - Toggle with `?`, shows by default
  - Name, version, repository
  - Description, size, install date/reason
  - URL, build date
  - Maintainer + votes (AUR packages)
  - Async fetching with 100ms debounce

## Phase 3: Depth & Safety ✓

- [x] **5. Arch news viewer** - Check archlinux.org news before updating (manual intervention warnings)
- [x] **6. Dependency view** - Show what depends on selected package before removing

## Phase 4: Maintenance Tools

- [x] **7. Cache cleanup** - Clear old package versions (`paccache -r`)
- [ ] **8. Export package list** - Backup of explicitly installed packages (`pacman -Qqe`)
- [ ] **9. Update confirmation** - Preview what will be installed/removed before running

## Phase 5: Nice to Have

- [ ] **Sort options** - Sort by name, size, date installed
- [ ] **Downgrade support** - Rollback packages via downgrade tool
- [ ] **System info header** - Kernel version, last update date
