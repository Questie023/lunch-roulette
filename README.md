# Lunch Roulette

[![CI](https://github.com/Questie023/lunch-roulette/actions/workflows/ci.yml/badge.svg)](https://github.com/Questie023/lunch-roulette/actions/workflows/ci.yml)

A Slack bot that decides where the team goes for lunch. The pick is random but fair.

Type `/lunch` in a channel and people press **Join**. The bot then picks a restaurant that is open, within walking distance and safe for everyone's dietary needs. Ratings and preferences nudge the odds a little, but no single person can steer the result.

It runs on Cloudflare Workers with D1, is written in Rust, and needs no server of your own.

## How a place is picked

1. **Filters.** A restaurant stays in the running only if it:
   - is active;
   - has not been shown yet in this session;
   - is open for at least 45 minutes of the lunch window;
   - is within the strictest participant's walking limit;
   - has no *known* conflict with anyone's dietary needs.

   Unknown diet information is never treated as safe: the place stays, with a warning to check with the staff.
2. **Group opinion.** Each participant gets an opinion of every candidate between −1 and 1. It is built from their latest rating (👍 +1, 🙂 0, 👎 −1), their favourites, the cuisines they like or dislike, and their price preference.
   - Opinions are centred on each person's own average, so someone who dislikes everything counts as neutral.
   - The group opinion is the **mean** over the people who joined.
   - A restaurant's weight is `2^opinion`, always between 0.5× and 2×.
3. **Variety.** A place chosen yesterday is ten times less likely today. The penalty fades out over two weeks.
4. **Weighted roulette.** One place is picked at random in proportion to its weight. The board explains the choice, for example "2 of 3 liked it last time", without naming anyone.

With eight places to choose from:
- each one has a 12.5% chance by default;
- one person in a group of four can raise their favourite to about 15%;
- a unanimous group can raise it to about 22%.

## What it does in Slack

- **`/lunch`** opens today's board for the channel, with these controls:
  - **Join** / **Leave**;
  - **🎲 Pick a place** and **🔄 Try another**;
  - **Pick by hand**;
  - **✅ Let's go**;
  - a **📋 Today's menu** link.
- **👍 Good choice / 🙂 Okay / 👎 Not for me** let people rate the place after lunch.
- **`/lunch add`** adds a restaurant: name, cuisine, walking time, price level, lunch hours and menu link.
- **Security and speed.** Every request is verified with Slack's signing secret. The bot acknowledges within Slack's three-second limit and does slow work in the background.

## Architecture

| Crate | Responsibility |
|---|---|
| `lunch-core` | Domain model, selection rules, the pick pipeline, the `Store` and `LunchService`. Pure Rust; builds for native and `wasm32`. |
| `lunch-sqlite` | `Db` implementation on SQLite (tests and future local tools) plus the migration runner. |
| `lunch-slack` | Request signature check, payload parsing and Block Kit rendering. No I/O. |
| `lunch-worker` | The Cloudflare Worker: routes Slack requests into `LunchService` over D1. |
| `tools/slack-sim` | Sends signed fake Slack requests to a local worker. |

All SQL lives in one place and runs unchanged on D1 and SQLite. A shared storage scenario checks both engines. See [docs/architecture.md](docs/architecture.md).

## Quick start (local, no Slack needed)

Prerequisites:
- Rust stable with the `wasm32-unknown-unknown` target (`rust-toolchain.toml` asks for it);
- Node.js, for `npx wrangler`.

```bash
git clone https://github.com/Questie023/lunch-roulette.git
cd lunch-roulette
cargo test

cd crates/lunch-worker
cp wrangler.example.toml wrangler.toml
cp .dev.vars.example .dev.vars
npx wrangler d1 migrations apply lunch-roulette --local
npx wrangler d1 execute lunch-roulette --local --file ../../seeds/example.sql
npx wrangler dev --port 8787
```

The first build installs `worker-build` and compiles the worker to WebAssembly, which takes a few minutes.

In a second terminal, from the repository root:

```bash
cargo run -p slack-sim -- lunch                 # open today's board
curl -s -X POST localhost:8787/__selftest       # run the storage scenario against local D1
```

`.dev.vars.example` turns on two development switches:
- `SLACK_DRY_RUN=1`: Slack API calls are logged in the `wrangler dev` output, not sent;
- `SELFTEST=1`: enables `/__selftest`.

## Deploy your own

See [docs/deployment.md](docs/deployment.md). You need:
- a Cloudflare account (the free plan is enough);
- permission to install apps in your Slack workspace.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo clippy -p lunch-worker --target wasm32-unknown-unknown -- -D warnings
```

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
