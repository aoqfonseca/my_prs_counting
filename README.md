# prs

A small Rust CLI that lists the pull requests **you merged** in a date range, across **GitHub** and **Forgejo** in one view. PRs carrying a bot label (default `evergreen`) are flagged so you can tell human work from bot-made PRs.

## Features

- Queries GitHub and Forgejo in parallel
- Date selection: today, yesterday, last 7 days, a single day, or a custom range
- Rich table output, plain text, or JSON
- Interactive terminal UI (`prs tui`) with keyboard navigation
- Bot PR detection through a configurable label

## Requirements

- [Rust](https://rustup.rs) (edition 2024, so a recent stable toolchain)
- **GitHub:** the [`gh` CLI](https://cli.github.com), authenticated (`gh auth login`)
- **Forgejo:** an API token and username (see [Configuration](#configuration))

## Installation

```bash
git clone <repo-url> && cd my_prs_counting
cargo install --path .
```

This installs the `prs` binary. To run without installing, use `cargo run --release -- <args>`.

## Usage

```bash
prs                          # PRs merged today (default)
prs --yesterday              # yesterday only
prs --week                   # last 7 days, including today
prs --date 2026-11-10        # a specific day
prs --since 2026-11-01 --until 2026-11-15   # a range (--until defaults to today)

prs --source github          # only GitHub (also: forgejo, all)
prs --json                   # machine-readable output
prs --plain                  # no rich formatting

prs tui                      # interactive UI, starts on today
prs tui --week               # interactive UI, starts on the last 7 days
```

Short flags: `-t` today, `-y` yesterday, `-w` week, `-d` date. These are mutually exclusive; `--since/--until` take precedence over the others.

Run `prs --help` for the full list.

### Terminal UI keys

| Key | Action |
| --- | --- |
| `j` / `↓`, `k` / `↑` | Move selection |
| `PageDown` / `PageUp` | Move by 10 |
| `g` / `Home`, `G` / `End` | Jump to top / bottom |
| `t`, `y`, `w` | Switch to today / yesterday / last 7 days |
| `r` | Reload |
| `o` / `Enter` | Open the selected PR in your browser |
| `q` / `Esc` / `Ctrl+C` | Quit |

## Configuration

Options can be set by flag or environment variable.

| Flag | Env var | Default | Description |
| --- | --- | --- | --- |
| `--github-user` | `GITHUB_USER` | `@me` | GitHub author to search for |
| `--forgejo-host` | `FORGEJO_HOST` | `git.carta.rocks` | Forgejo hostname |
| `--forgejo-user` | `FORGEJO_USER` | from `tea` login | Forgejo username |
| `--bot-label` | `BOT_LABEL` | `evergreen` | Label that marks bot-made PRs (case-insensitive) |
| n/a | `FORGEJO_TOKEN` or `GITEA_TOKEN` | from `tea` login | Forgejo API token |

If no token is set, `prs` falls back to the login stored by the [`tea` CLI](https://gitea.com/gitea/tea) (`tea login add`).

**Never commit tokens.** Keep them in your shell environment or in a `.env` file, which is git-ignored.

## Output and errors

Each source reports independently: if one fails (for example `gh` is missing or the Forgejo token is not set), its error is shown and the other source's results are still printed. In `--json` mode the output is a list of `{ "source", "error", "prs" }` objects.

## Project layout

| File | Purpose |
| --- | --- |
| `src/main.rs` | CLI parsing and output selection |
| `src/lib.rs` | Data model, `Config`, parallel fetching, bot marking |
| `src/range.rs` | Date range resolution |
| `src/github.rs` | GitHub source (via `gh api`) |
| `src/forgejo.rs` | Forgejo source (Gitea-compatible API) |
| `src/render.rs` | Rich and plain output |
| `src/tui.rs` | Interactive terminal UI (ratatui) |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
