# woler

Browse, search, and remove installed Arch Linux packages — all from your terminal.

## Install

### With Rust toolchain
```bash
cargo install --git https://github.com/YOUR_USER/woler
```

### AUR (once submitted)
```bash
yay -S woler
```

### Pre-built binary
```bash
curl -sSL https://github.com/YOUR_USER/woler/releases/latest/download/woler-x86_64-linux.tar.gz | tar xz -C ~/.local/bin
```

## Usage

```
woler                # Open the TUI browser
woler list           # List all packages
woler list --apps    # GUI applications only
woler list --clis    # CLI tools only
woler list --libs    # Libraries only
woler list -s firefox  # Search by name/description
woler remove firefox   # Remove a package
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
