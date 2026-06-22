# woler

Browse, search, and remove installed Arch Linux packages — all from your terminal.

## Install

### With Rust toolchain
```bash
cargo install --git https://github.com/subhradeepsarkae-ai/woler
```

### AUR (once submitted)
```bash
yay -S woler
```

### Pre-built binary
```bash
# One-liner
curl -fsSL https://raw.githubusercontent.com/subhradeepsarkae-ai/woler/main/install.sh | sh

# Or manual download
curl -sSL https://github.com/subhradeepsarkae-ai/woler/releases/latest/download/woler-x86_64-linux.tar.gz | tar xz -C ~/.local/bin
```

## Usage

```
woler                # Open the TUI browser
woler --app          # TUI filtered to Apps (with delete)
woler --cli          # TUI filtered to CLIs (with delete)
woler --lib          # TUI filtered to Libs (with delete)
woler list           # List all packages to stdout
woler list --apps    # GUI apps to stdout
woler list --clis    # CLI tools to stdout
woler list --libs    # Libraries to stdout
woler list -s <term> # Search by name/description
woler remove <pkg>   # Remove a package
woler refresh        # Force cache refresh
```

### TUI keybindings

| Key | Action |
|---|---|
| `j`/`k` or `↑`/`↓` | Navigate list |
| `Tab` or `1`-`4` | Switch tab |
| `/` | Search/filter |
| `Enter` | Show package details |
| `d` | Delete (with confirmation) |
| `D` | Delete + orphan cleanup |
| `r` | Force refresh |
| `?` | Show help |
| `q` | Quit |

## How it works

Directly reads `/var/lib/pacman/local/*/desc` and `files` — no `pacman` spawning. Scans `.desktop` files in `/usr/share/applications/` to detect GUI apps.

Results are cached in `~/.cache/woler/packages.msgpack` for instant subsequent launches.
