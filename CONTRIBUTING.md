# Contributing

Thanks for helping! Issues and pull requests are welcome.

## Setup

- **Rust:** stable. `rust-toolchain.toml` pins `stable` plus the `wasm32-unknown-unknown` target.
- **Node.js:** only for `npx wrangler` when you run the worker locally.

[README.md](README.md#quick-start-local-no-slack-needed) shows how to run the bot locally without Slack.

## Before you open a pull request

CI runs exactly these:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo clippy -p lunch-worker --target wasm32-unknown-unknown -- -D warnings
```

## Guidelines

- **Tests first.** Write a failing test, then make it pass. Most logic is pure and lives in `lunch-core` or `lunch-slack`, so it is easy to test natively.
- **Snapshots.** Block Kit output is covered by `insta` snapshots in `crates/lunch-slack/tests/snapshots`. After an intentional change, run `INSTA_UPDATE=always cargo test -p lunch-slack` and review the diff before committing.
- **SQL.**
  - SQL lives only in `crates/lunch-core/src/sql.rs` and must run unchanged on SQLite and Cloudflare D1.
  - Use numbered parameters (`?1`).
  - Do not use `BEGIN`, `SAVEPOINT` or `PRAGMA`: D1 rejects them, and `Db::batch` is the atomic unit.
- **Migrations.**
  - Add a new numbered file in `migrations/` and register it in `crates/lunch-sqlite/src/lib.rs`.
  - Never edit or rename a migration that has been released: D1 tracks them by file name.
- **Storage changes.** Extend `lunch_core::testkit::store_scenario`, so that SQLite (`cargo test`) and D1 (`POST /__selftest` on `wrangler dev`) are held to the same behaviour.
- **Privacy.** Explanations shown in Slack must stay aggregated: never name who has which dietary need or who disliked a place.
- **Commits.** Use the conventional style (`feat:`, `fix:`, `docs:`, `chore:`…). Write messages that explain why.

## Reporting bugs

Open an issue with steps to reproduce. Include what you expected and what happened, and the relevant `wrangler tail` output if there is one. For security issues, see [SECURITY.md](SECURITY.md).
