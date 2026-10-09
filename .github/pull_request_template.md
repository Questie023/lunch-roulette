## What and why

## How it was tested

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `cargo clippy -p lunch-worker --target wasm32-unknown-unknown -- -D warnings`
- [ ] Storage changes: `store_scenario` extended and `POST /__selftest` passes on `wrangler dev`
