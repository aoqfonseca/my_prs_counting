# Contributing

Thanks for helping improve `prs`.

## Getting started

1. Install a recent stable Rust toolchain via [rustup](https://rustup.rs).
2. Clone the repo and build:
   ```bash
   cargo build
   ```
3. Try it out:
   ```bash
   cargo run -- --week
   cargo run -- tui
   ```

You'll need `gh` authenticated and/or Forgejo credentials to see real data. See the [README](README.md#configuration).

## Workflow

1. Create a branch from `main`: `git checkout -b feature/short-description`
2. Make your change in small, focused commits.
3. Before pushing, make sure all of these pass:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```
4. Push and open a pull request against `main`.

## Pull requests

Include in the description:

- **Link to the JIRA ticket** (if one exists), at the top
- **Description:** the problem, the solution, and the files or functions changed
- **Test plan:** manual steps, automated tests added, and edge cases checked

Keep PRs small and focused; unrelated changes belong in separate PRs.

## Code guidelines

- Core logic (data model, date ranges, fetchers) lives in `src/lib.rs` and its modules. Keep `main.rs` limited to CLI wiring.
- Sources return a `SourceResult` rather than failing the whole run, so one broken source never hides the others.
- Use `anyhow` for errors in fallible helpers.
- Add unit tests for pure logic such as date-range resolution and timestamp handling.
- Follow `rustfmt` defaults and keep Clippy clean.

## Adding a new source

1. Add `src/<source>.rs` with a `fetch(...) -> SourceResult` function.
2. Declare it in `src/lib.rs` and add a flag to `Config`.
3. Spawn it in `fetch_all` alongside the others.
4. Add the matching `--source` value and options in `src/main.rs`.
5. Document new flags and env vars in the README.

## Security

- Never commit tokens, `.env` files, or credentials (they are git-ignored; keep it that way).
- Don't log or print token values.
- Report security concerns privately to the maintainers rather than in a public issue.
