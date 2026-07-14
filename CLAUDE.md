# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

A Rust CLI tool that fetches a GitHub user's profile, repos, and activity via the GitHub API and generates a profile README (Markdown). See AGENTS.md for the original spec.

## Commands

```bash
cargo build                       # build
cargo check                       # fast type-check
cargo run -- <username>           # generate README to stdout
cargo run -- <username> -o README.md --token <PAT>   # write to file, authenticated
cargo run -- <username> --push --token <PAT>         # push to {username}/{username} repo
cargo fmt && cargo clippy         # format and lint
```

- Token can also come from the `GITHUB_TOKEN` env var (clap `env` feature).
- Rust toolchain is managed by mise (`mise.toml`, rust = latest). Edition 2024.
- There are no tests currently.

## Architecture

Three modules forming a fetch → transform → render pipeline:

- `src/main.rs` — CLI (clap derive) and orchestration. Fetches user/repos/events concurrently with `tokio::try_join!`, builds a daily contribution map, then renders and optionally writes/pushes.
- `src/github.rs` — `GitHubClient` wrapping reqwest. All REST calls go through `auth_header()` which attaches the token only if present. Also holds the GraphQL contribution-calendar query and the `push_readme` (GitHub Contents API: fetch SHA, then PUT) logic.
- `src/template.rs` — pure rendering: takes the fetched data and returns the full Markdown string. Contains the streak calculator, ASCII contribution "grass" grid, activity feed formatter, and language bar chart.

### Token-dependent behavior (key design point)

The tool degrades gracefully without a token, and several things branch on `client.has_token()`:

- **Cutoff window**: 180 days with a token, 90 without (set in `main.rs`).
- **Contribution data**: with a token, the GraphQL contribution calendar (accurate, all contribution types) is used; without, it falls back to counting `PushEvent`s per day from the REST Events API (best effort). GraphQL failure also falls back to events.
- **`--push` requires a token** (repo/public_repo scope); it errors otherwise.

### Data-flow details worth knowing

- The Events API truncates PR payloads, so `main.rs` makes follow-up `get_pr_title` calls for each unique `(repo, PR number)` pair and passes a `pr_titles` map into the renderer.
- `get_events` pages through at most 10 pages of 30 events, stopping early once a page's oldest event is past the cutoff.
- The activity feed (`latest_activities`) dedups events: PR/issue events by `(kind, repo, action)`, everything else by `(kind, repo)`, and filters out low-signal events (e.g. `WatchEvent`, PR `synchronize` actions).
- In `PushEvent` payloads, `size` is the authoritative commit count; the `commits` array may be truncated.
