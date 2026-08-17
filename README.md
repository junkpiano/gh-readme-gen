# gh-readme-gen

Generate a GitHub **profile README** from your profile, repos and public activity — and push it
straight to your `{username}/{username}` repo. Available as a GitHub Action and as a standalone
Rust CLI.

The generated README includes a header with your bio and links, current/longest commit streaks, a
top-languages bar chart, pinned-style top repos, and a recent activity feed with real PR and issue
titles.

## Use as a GitHub Action

Add this to your profile repository (`{username}/{username}`) as
`.github/workflows/profile-readme.yml`:

```yaml
name: Update profile README

on:
  schedule:
    - cron: '17 3 * * *'   # daily
  workflow_dispatch:

permissions:
  contents: write

jobs:
  update:
    runs-on: ubuntu-latest
    steps:
      - uses: junkpiano/gh-readme-gen@v0
```

That's it — with no inputs the action generates the README for the repository owner and pushes it
to `{owner}/{owner}/README.md` using the workflow's `GITHUB_TOKEN`. The generated Markdown is also
appended to the job summary so you can review each run.

### Inputs

| Input      | Default                          | Description |
| ---------- | -------------------------------- | ----------- |
| `username` | `${{ github.repository_owner }}` | User to generate the README for. |
| `token`    | `${{ github.token }}`            | Token for the API and for pushing. See [Tokens](#tokens). |
| `push`     | `true`                           | Push to `{username}/{username}`. Set `false` to only generate. |
| `output`   | *(temp file)*                    | Where to write the Markdown. Always reported via `readme-path`. |
| `version`  | `0.1.0`                          | Release of `gh-readme-gen` to run, or `latest`. |
| `summary`  | `true`                           | Append the generated README to the job summary. |

### Outputs

| Output        | Description |
| ------------- | ----------- |
| `readme-path` | Path to the generated Markdown file. |
| `commit-url`  | URL of the pushed README. Empty when `push` is disabled. |

### Generate without pushing

Useful if you'd rather commit the file yourself, or want to open a PR instead:

```yaml
      - uses: actions/checkout@v4
      - uses: junkpiano/gh-readme-gen@v0
        with:
          push: false
          output: README.md
      - uses: stefanzweifel/git-auto-commit-action@v5
        with:
          commit_message: 'chore: update profile README'
```

### Tokens

- The default `GITHUB_TOKEN` works when the workflow **runs inside the profile repo** and the job
  grants `permissions: contents: write`.
- To run the workflow from a *different* repo (or to include private contributions in the streak
  counts), pass a PAT with `repo` (or `public_repo`) scope:

  ```yaml
      - uses: junkpiano/gh-readme-gen@v0
        with:
          token: ${{ secrets.PROFILE_README_TOKEN }}
  ```

Commits made with the default `GITHUB_TOKEN` do not trigger further workflows; the push message
also carries `[skip ci]`.

### Runners

The action downloads a prebuilt binary for the runner it's on — no Rust toolchain needed. Supported:
Linux (x86_64, arm64), macOS (Intel, Apple Silicon) and Windows (x86_64).

## Use as a CLI

```bash
cargo install --git https://github.com/junkpiano/gh-readme-gen
```

```bash
gh-readme-gen <username>                              # print to stdout
gh-readme-gen <username> -o README.md                 # write to a file
gh-readme-gen <username> --push --token <PAT>         # push to {username}/{username}
```

The token can also come from the `GITHUB_TOKEN` environment variable. Without a token the tool
still works: it looks back 90 days instead of 180 and approximates contributions from public push
events rather than the GraphQL contribution calendar. `--push` requires a token.

## Development

```bash
cargo check                  # fast type-check
cargo fmt && cargo clippy    # format and lint
cargo run -- <username>      # run against a real account
```

### Cutting a release

The action resolves its binary from a GitHub Release, so a new version means a new tag:

1. Bump `version` in `Cargo.toml` **and** the `version` input default in `action.yml` — the release
   workflow fails if either disagrees with the tag.
2. `git tag v0.2.0 && git push origin v0.2.0`.

`.github/workflows/release.yml` then cross-builds every target, uploads the tarballs plus SHA-256
checksums, publishes the release and moves the `v0` major tag so `@v0` picks it up.

## License

MIT
