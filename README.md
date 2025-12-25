# upkeep

A terminal user interface (TUI) for managing Arch Linux system updates, installed packages, orphans, and rebuild issues.

![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)
![Arch Linux](https://img.shields.io/badge/Arch-Linux-1793D1?logo=arch-linux)

## Features

- **Updates Tab** - View and install pending pacman and AUR updates
- **Installed Tab** - Browse explicitly installed packages, uninstall or reinstall
- **Orphans Tab** - Find and remove packages no longer needed as dependencies
- **Rebuilds Tab** - Detect and fix ABI/version mismatch issues (e.g., after Python/Qt updates)
- **Filtering** - Filter package lists by name on Updates and Installed tabs
- **Batch Operations** - Select multiple packages with Space, select all/none with a/n

## Installation

### From source

```bash
git clone https://github.com/yourusername/upkeep.git
cd upkeep
cargo build --release
sudo cp target/release/upkeep /usr/local/bin/
```

### Dependencies

- Rust 1.70+
- An AUR helper (`yay` by default, configurable)

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
| `r` | Refresh current tab |
| `q` | Quit |

#### Updates Tab
| Key | Action |
|-----|--------|
| `u` | Update selected packages |
| `Enter` | Update all packages |

#### Installed Tab
| Key | Action |
|-----|--------|
| `d` | Remove package(s) |
| `D` | Remove with dependencies |
| `i` | Reinstall package(s) |
| `I` | Reinstall from source (AUR rebuild) |

#### Orphans Tab
| Key | Action |
|-----|--------|
| `d` | Remove package(s) |
| `D` | Remove with dependencies |

#### Rebuilds Tab
| Key | Action |
|-----|--------|
| `Enter` | Run rebuild command |

## Configuration

Configuration files are stored in `~/.config/upkeep/`.

### config.toml

```toml
# AUR helper to use (default: yay)
aur_helper = "yay"
```

### checks.toml

Define custom rebuild checks for applications that break after system updates:

```toml
[[check]]
name = "obs-studio"
command = ["timeout", "3", "obs", "--help"]
error_patterns = ["ABI mismatch", "symbol lookup error"]
rebuild = "yay -S --rebuild obs-studio"

[[check]]
name = "my-aur-package"
command = ["timeout", "3", "my-app", "--version"]
error_patterns = ["plugin was built with a different version"]
rebuild = "yay -S --rebuild my-aur-package"
```

## Roadmap

See [TODO.md](TODO.md) for planned features.

## License

MIT
