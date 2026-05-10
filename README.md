# faceitui

> Terminal-based FACEIT CS2 statistics viewer

`faceitui` is a TUI (Terminal User Interface) tool for viewing FACEIT CS2 player statistics right from your terminal.

<p align="center">
  <img src="EDE6367A-A5E3-4BE4-9EDA-F4B320588A59.png" width="700" alt="faceitui screenshot">
</p>

## Features

- **Player Search** — search by nickname or player ID
- **Lifetime Stats** — total matches, K/D, win rate, headshot %, and more
- **Match History** — last 30 matches with detailed view, pagination (`n`/`p`)
- **Map Performance** — per-map win/loss breakdown and detailed stats
- **AI Analysis** — OpenCode AI powered player strengths/weaknesses analysis
- **Themes** — 20+ color themes, switch with `t`
- **Export** — JSON export for any tab (`e`)
- **Mouse & Keyboard** — full mouse support, scroll, vim-like navigation

## Install

```bash
npm install -g faceitui --foreground-scripts
```

> **Note:** On Windows, `--foreground-scripts` is required to see the install banner. After the first install, this is configured automatically for future updates.

## Usage

```bash
faceitui
```

### Shortcuts

| Key | Action |
|-----|--------|
| `i` | Enter search mode |
| `Enter` | Submit search |
| `Esc` | Clear / go home |
| `Tab` | Switch tabs (Lifetime / Matches / Maps) |
| `↑` `↓` | Navigate rows |
| `PgUp` `PgDn` | Page scroll |
| `r` | Refresh |
| `a` | AI analysis |
| `e` | Export to JSON |
| `c` | Copy player ID to clipboard |
| `t` | Theme selector |
| `n` / `p` | Next / previous match page |
| `?` | Help |
| `q` | Quit |

### AI Key

On first AI usage, you'll be prompted to enter your [OpenCode AI](https://opencode.ai) API key. The key is kept in memory for the session and never written to disk.

You can also set it via environment variable:

```bash
# Linux/macOS
export AI_API_KEY="sk-..."

# Windows
set AI_API_KEY=sk-...
```

## FACEIT API

The FACEIT API key is embedded in the release binary. For custom builds, set `FACEIT_API_KEY` at compile time or in a `config.toml`:

```toml
api_key = "your-faceit-api-key"
```

## Development

```bash
git clone https://github.com/cnaltn/Faceitui.git
cd Faceitui
cargo build --release
```

### Requirements

- Rust stable
- Node.js (for npm packaging only)

## License

MIT
