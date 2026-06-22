# woler — Checkpoint

## Project Status

**Complete.** Fast TUI/CLI tool for browsing, searching, and removing installed Arch Linux packages. Reads pacman DB directly (no subprocesses), caches results via MsgPack for ~15ms subsequent launches.

## Files Created

| File | Size | Description |
|------|------|-------------|
| `src/main.rs` | ~7 KB | CLI entry: list/--app/--cli/--lib, remove, refresh, TUI dispatch |
| `src/app.rs` | ~23 KB | ratatui TUI: tabs, search, detail panel, delete modal |
| `src/db.rs` | ~9 KB | Pacman DB parser — reads `/var/lib/pacman/local/*/desc` and `files` directly |
| `src/cache.rs` | ~2 KB | MsgPack cache layer at `~/.cache/woler/packages.msgpack` |
| `Cargo.toml` | ~0.5 KB | Dependencies: ratatui, crossterm, serde, rmp-serde, clap, chrono |
| `install.sh` | ~2 KB | One-liner installer — downloads pre-built binary or falls back to cargo |
| `README.md` | ~2 KB | Full documentation with install, usage, keybindings |
| `.gitignore` | ~20 B | Ignores target/, *.pyc, *.swp |
| `checkpoint.md` | — | This file |

## Binary Specs

- **Size:** 967KB (stripped release, LTO, opt-level=z)
- **Memory:** ~5MB for 2000 packages
- **First launch:** ~150ms (parse pacman DB, write cache)
- **Subsequent:** ~15ms (read cache, skip if DB unchanged)

## CLI Design

```
woler                 Open the TUI browser
woler list            List all packages to stdout
woler list --apps     GUI applications only
woler list --clis     CLI tools only
woler list --libs     Libraries only
woler list -s <term>  Search by name/description
woler --app           Short for: list --apps
woler --cli           Short for: list --clis
woler --lib           Short for: list --libs
woler remove <pkg>    Remove a package (sudo pacman -Rns)
woler refresh         Force rebuild cache
```

## TUI Keybindings

| Key | Action |
|---|---|
| `j`/`k` or `↑`/`↓` | Navigate list |
| `Tab` / `Shift+Tab` | Cycle tabs |
| `1` `2` `3` `4` | Direct tab: All / Apps / CLIs / Libs |
| `/` | Enter search mode |
| `Enter` | Show package details |
| `Esc` | Close detail / cancel search |
| `d` | Delete package (with confirmation) |
| `D` | Delete + orphan cleanup |
| `r` | Force refresh |
| `?` | Show help in status bar |
| `q` | Quit |

## Category Logic

| Category | Detected by |
|---|---|
| **App** | Package owns files in `/usr/share/applications/*.desktop` |
| **CLI** | Package owns files in `/usr/bin/`, `/usr/local/bin/`, or `/usr/sbin/` |
| **Library** | Neither (no .desktop, no bins) |

Note: A package can be both **App** and **CLI** (e.g. firefox, kitty). The `--app` filter shows all GUI packages; `--cli` shows all packages with binaries (including GUI ones).

## Data Sources (world-readable, no sudo needed)

| Source | Location |
|---|---|
| Package metadata | `/var/lib/pacman/local/*/desc` |
| Owned files | `/var/lib/pacman/local/*/files` |
| GUI detection | `/usr/share/applications/*.desktop` |
| Cache | `~/.cache/woler/packages.msgpack` |

## Git Status

- Local git repo at `/home/bro/Projects/woler`
- Branch: `main`
- Remote: `https://github.com/subhradeepsarkae-ai/woler.git`
- Pushed: ✅ (3 commits)

## Install

```bash
# One-liner (pre-built binary, no Rust needed)
curl -fsSL https://raw.githubusercontent.com/subhradeepsarkae-ai/woler/main/install.sh | sh

# Or with cargo
cargo install --git https://github.com/subhradeepsarkae-ai/woler
```

## Test Results

```
woler list           → 934 packages listed
woler list --apps    → 52 GUI apps
woler list --clis    → 438 CLI tools
woler list --libs    → 491 libraries
woler list -s vim    → 5 matches
woler --app          → 52 GUI apps (top-level flag)
woler refresh        → rescanned and cached
woler (TUI)          → renders with all features
```
