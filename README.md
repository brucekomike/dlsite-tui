# dlsite-tui

A cross-platform terminal UI for browsing and downloading your [DLsite](https://www.dlsite.com) purchased library. Rust rewrite of [dlsite-bash](https://github.com/brucekomike/dlsite-bash).

## Features

- **Login** – authenticates with DLsite using cookies (persisted across sessions)
- **Fetch Library** – syncs your purchased works into a local SQLite database (`info.db`)
- **Browse by ID** – scrollable list of all works, select one to download
- **Browse by Circle** – group works by maker/circle, download one or all
- **Browse by Recent** – works sorted by purchase date (newest first)
- **Download All** – batch-download every work in the database

## Installation

Download a pre-built binary for your platform from the [Releases](../../releases) page:

| Platform | Binary |
|----------|--------|
| Linux (x86-64) | `dlsite-tui-linux-x86_64` |
| macOS (Intel) | `dlsite-tui-macos-x86_64` |
| macOS (Apple Silicon) | `dlsite-tui-macos-aarch64` |
| Windows (x86-64) | `dlsite-tui-windows-x86_64.exe` |

### Build from source

```sh
cargo build --release
# binary: target/release/dlsite-tui
```

Requires Rust stable ≥ 1.70.

## Usage

Run `dlsite-tui` from the directory where you want `info.db` and `downloads/` to be created.

```sh
./dlsite-tui
```

**Keyboard shortcuts:**

| Key | Action |
|-----|--------|
| `↑` / `↓` | Move selection |
| `Enter` | Confirm / select |
| `Esc` | Go back |
| `q` | Quit |
| `Ctrl+C` | Force quit |

**First run:**
1. Enter your DLsite username and password on the login screen.
2. From the main menu, press **Fetch Library** to sync your purchase list and work metadata.
3. Browse and download works using the menu options.

Downloads are saved to `downloads/<circle>/<work-name>/`.

## Data files

| File | Description |
|------|-------------|
| `info.db` | SQLite database of your purchased works |
| `{config_dir}/dlsite-tui/cookies.json` | Saved session cookies |
| `{config_dir}/dlsite-tui/config.json` | Saved username |

`config_dir` is `~/.config/dlsite-tui` on Linux, `~/Library/Application Support/dlsite-tui` on macOS, and `%APPDATA%\dlsite-tui` on Windows.

## CI

GitHub Actions builds binaries for Linux, macOS (x86-64 + ARM64), and Windows on every push. Release artifacts are attached automatically when a `v*` tag is pushed.
