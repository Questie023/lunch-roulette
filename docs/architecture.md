# Architecture

## Overview

```
Slack ──HTTPS──▶ lunch-worker (Cloudflare Worker, wasm32)
                   │  verify signature, parse payload        (lunch-slack)
                   │  LunchService<D1Db, SystemClock, StdRng> (lunch-core)
                   │  render Block Kit, call Slack Web API     (lunch-slack + slack_client)
                   ▼
                  D1 (SQLite at the edge)
```

The domain lives in `lunch-core`, which has no I/O. Persistence goes through a two-method trait, so the same `Store` and `LunchService` run in two places:
- **in the worker**, over Cloudflare D1;
- **natively**, over SQLite, in tests and any future local tool.

## Crates

| Crate | Depends on | Notes |
|---|---|---|
| `lunch-core` | jiff, rand, serde | `model`, `rules`, `pipeline`, `sql`, `rows`, `store`, `service`, `testkit` (feature). |
| `lunch-sqlite` | lunch-core, rusqlite | `SqliteDb` implements `Db`; `migrate` applies `migrations/*.sql` tracked by `PRAGMA user_version`. |
| `lunch-slack` | lunch-core, hmac, sha2 | `verify` (v0 signatures), `payload`, `actions`, `blocks`, `modal`. Pure functions, unit and snapshot tested. |
| `lunch-worker` | lunch-core, lunch-slack, worker | `D1Db`, Slack Web API client, HTTP handlers. Built with `worker-build`. |
| `tools/slack-sim` | lunch-slack, ureq | Signs and sends fake slash commands, button clicks and modal submissions. |

## Requests

**Common to every Slack route:**
1. Every `/slack/*` request is checked with Slack's v0 signature: HMAC-SHA256 over `v0:{timestamp}:{body}`, constant-time comparison, ±5 minutes.
2. The worker answers within Slack's three-second limit.
3. Slow work (Slack API calls, re-rendering the board) runs in `ctx.wait_until`.
4. Failures are logged and shown to the user as an ephemeral message.

**Per request type:**
- **`/lunch`** opens (or reuses) the session for the channel and day, then posts the board and stores its message timestamp. Running `/lunch` again moves the board down. The old message turns into a pointer.
- **Button clicks** carry the session id, plus the pick or restaurant id where needed. A click on an older board therefore still acts on the right session and updates the clicked message.
- **Modal submissions** (`/lunch add`) are validated synchronously, so field errors keep the modal open.

## Pick pipeline (`lunch-core::pipeline::run_pick`)

1. **Pool:** active restaurants, minus those already shown in this session.
2. **Opening hours** (`rules::opening_hours`): a subset of the OpenStreetMap syntax. A place is open when it is open for at least 45 minutes of the lunch window. Unknown hours keep the place, with a ⏰ warning.
3. **Distance** (`geo`): explicit walking time, or haversine × 1.3 at 80 m/min. The limit is the strictest participant's.
4. **Diet** (`rules::constraints`): per tag `Yes`/`No`/unknown. Today's offers can satisfy a tag, and vegan implies vegetarian, milk-free and egg-free.
   - a known conflict removes the place;
   - unknown keeps it with a warning and does not change its weight.
5. **Group opinion** (`rules::score`): each participant's opinion in [−1, 1] is centred on their own average. The group opinion is the mean over participants, and the weight is `2^g` ∈ [0.5, 2]. `pipeline` clamps the weight to that range in case of a bug.
6. **Variety** (`rules::variety`): linear between the points (days → factor) 0 → 0.05, 1 → 0.1, 2 → 0.4, 7 → 0.85, 14 → 1.0.
7. **Roulette** (`rules::roulette`): weighted choice. Non-positive and non-finite weights never win.

If a filter empties the pool, the board says which one did. Only distance can be ignored, with the **🚶 Ignore distance** button. Dietary needs are never relaxed.

## Storage

```rust
trait Db {
    async fn all<T: DeserializeOwned>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>>;
    async fn batch(&self, stmts: Vec<Stmt>) -> Result<()>; // atomic
}
```

- **One copy of every query.** All SQL lives in `lunch-core/src/sql.rs` and uses numbered parameters (`?1`). Rows are deserialised by column name into the structs in `rows.rs`.
- **D1 differences** are absorbed in `D1Db`:
  - no `BEGIN` (D1 `batch` is atomic instead);
  - integers are bound as JavaScript numbers (exact below 2^53);
  - booleans are read as `0`/`1`.
- **Migrations** in `migrations/` are append-only and **never renamed**. D1 records applied migrations by file name, and `lunch-sqlite` by count.
- **One scenario, both engines.** `testkit::store_scenario` exercises every `Store` method. It runs natively against SQLite in `cargo test`, and inside the worker against D1 at `POST /__selftest` (only when `SELFTEST=1`).

## Data model

| Table | Purpose |
|---|---|
| `settings` | Office location, time zone, lunch window, default walking limit, currency. |
| `users` | Slack user id, display name, personal walking limit and price preference. |
| `user_constraints` | Dietary needs (allergens and diets). |
| `user_cuisine_prefs` | Cuisine likes and dislikes, from −2 to 2. |
| `user_restaurant_prefs` | Favourite restaurants. |
| `restaurants`, `restaurant_diet_support` | Places, and what each one states about diets. |
| `lunch_offers` | Dishes of the day, when known. |
| `lunch_sessions`, `session_participants` | One session per channel and day, and who joined it. |
| `session_picks` | Every roll or manual pick with its explanation; `accepted` marks where the group went. |
| `feedback` | One rating per person, session and place. The latest rating per person and place is their opinion. |

## Privacy

- Explanations are aggregated, such as "3 people like thai food". They never name anyone, and dietary warnings carry no names or counts.
- Participant lists show display names, never `@mentions`.
- Only people who joined today's session influence the pick.
