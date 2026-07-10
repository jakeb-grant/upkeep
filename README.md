# upkeep

A terminal user interface (TUI) for managing Arch Linux system updates, installed packages, orphans, and rebuild issues.

[![AUR](https://img.shields.io/aur/version/upkeep-git?label=AUR&logo=archlinux)](https://aur.archlinux.org/packages/upkeep-git)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Arch updates are easy until they aren't — a Python upgrade breaks half your AUR packages, an orphan you forgot about pulls in a dependency chain, and the news post you didn't read needed manual intervention. upkeep puts all of that in one TUI so you can see what's broken, what's stale, and what needs attention before you hit Enter.

## Features

- **Updates Tab** - View and install pending pacman and AUR updates
- **Installed Tab** - Browse explicitly installed packages, uninstall or reinstall
- **Orphans Tab** - Find and remove packages no longer needed as dependencies
- **Rebuilds Tab** - Detect and fix ABI/version mismatch issues (e.g., after Python/Qt updates) and track custom-patched packages against upstream
- **Search Tab** - Search and install packages from official repos and AUR
- **News Tab** - View Arch Linux news with smart highlighting:
  - `!` (yellow) - Items requiring manual intervention
  - `*` (blue) - Items related to your installed packages
- **Info Pane** - Toggle detailed package/article info with `?` key (works on all tabs)
- **Filtering** - Filter package lists by name on Updates and Installed tabs
- **Batch Operations** - Select multiple packages with Space, select all/none with a/n

## Installation

### AUR (recommended)

```bash
yay -S upkeep-git
```

### Prebuilt binary

Download from [GitHub Releases](https://github.com/jakeb-grant/upkeep/releases):

```bash
tar -xzf upkeep-v*.tar.gz
sudo mv upkeep /usr/local/bin/
```

### From source

```bash
git clone https://github.com/jakeb-grant/upkeep.git
cd upkeep
cargo build --release
sudo cp target/release/upkeep /usr/local/bin/
```

### Dependencies

- An AUR helper (`yay` by default, configurable)
- `pacman-contrib` — provides `checkupdates` (update checking) and `paccache` (cache cleanup)
- `wl-clipboard` or `xclip` (optional) — clipboard export of package lists

## Usage

```bash
upkeep
```

### Keybindings

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Switch tabs |
| `j` / `k` or `↓` / `↑` | Navigate list |
| `Space` | Toggle selection |
| `a` / `n` | Select all / none |
| `f` | Enter filter mode (Updates/Installed) |
| `F` or `Esc` | Exit filter mode |
| `?` | Toggle info pane |
| `r` | Refresh current tab |
| `q` | Quit |

#### Updates Tab
| Key | Action |
|-----|--------|
| `u` | Update selected packages |
| `Enter` | Update all packages |
| `c` | Clean package cache |

#### Installed Tab
| Key | Action |
|-----|--------|
| `d` | Remove package(s) |
| `D` | Remove with dependencies |
| `i` | Reinstall package(s) |
| `I` | Reinstall from source (AUR rebuild) |
| `c` | Export package lists to files |
| `C` | Copy package list to clipboard |

#### Orphans Tab
| Key | Action |
|-----|--------|
| `d` | Remove package(s) |
| `D` | Remove with dependencies |

#### Rebuilds Tab
| Key | Action |
|-----|--------|
| `Enter` | Run rebuild command |

#### Search Tab
| Key | Action |
|-----|--------|
| `Type` | Search packages |
| `Enter` | Install selected |
| `Esc` | Clear search |

#### News Tab
| Key | Action |
|-----|--------|
| `Shift+↑` / `Shift+↓` | Scroll article |
| `PgUp` / `PgDn` | Scroll article (fast) |
| `r` | Refresh news |

## Configuration

Configuration files are stored in `~/.config/upkeep/`.

### config.toml

```toml
# AUR helper to use (default: yay)
aur_helper = "yay"
```

### checks.toml

Define rebuild checks for applications that break after system updates:

```toml
# Error-pattern check: runs a command and looks for error strings in stderr
[[check]]
name = "obs-studio"
command = ["timeout", "3", "obs", "--help"]
error_patterns = ["ABI mismatch", "symbol lookup error"]
rebuild = "yay -S --rebuild obs-studio"
```

You can also track custom-patched packages against their upstream counterparts. When the upstream version is newer than your installed version, the check triggers:

```toml
# Version-track check: compares installed version against official repo
[[check]]
name = "libadwaita-custom"
tracks = "libadwaita"
rebuild = "libadwaita-rebuild bump"
```

All configured checks are shown in the Rebuilds tab with their current status (passing or triggered).

## License

MIT
