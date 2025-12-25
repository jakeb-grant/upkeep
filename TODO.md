# Upkeep Feature Roadmap

## Phase 1: Quick Wins

- [x] **1. Filter on Updates tab** - Consistency with Installed tab, reuse existing code
- [x] **2. Orphan packages tab** - Packages no longer needed as deps (`pacman -Qdt`), easy cleanup

## Phase 2: Core Feature

- [ ] **3. Package search tab** - Search across repos + AUR for new packages to install
  - `pacman -Ss <query>` for official repos
  - AUR RPC search endpoint for AUR packages
  - Show: name, version, description, repo/AUR source
  - Select and install with configured helper

## Phase 3: Depth & Safety

- [ ] **4. Package info popup** - Show description, size, dependencies, install date on `?` or `Enter`
- [ ] **5. Arch news viewer** - Check archlinux.org news before updating (manual intervention warnings)

## Phase 4: Polish

- [ ] **6. Sort options** - Sort by name, size, date installed
- [ ] **7. Dependency view** - Show what depends on selected package before removing
- [ ] **8. Cache cleanup** - Clear old package versions (`paccache -r`)

## Phase 5: Nice to Have

- [ ] **Export package list** - Backup of explicitly installed packages
- [ ] **Downgrade support** - Rollback packages via downgrade tool
- [ ] **System info header** - Kernel version, last update date
