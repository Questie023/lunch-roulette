# Deployment

You need:
- a Cloudflare account (the free plan is enough);
- a Slack workspace where you may install apps;
- Rust stable with the `wasm32-unknown-unknown` target;
- Node.js, for `npx wrangler`.

Run every `wrangler` command from `crates/lunch-worker`.

## 1. Cloudflare

```bash
cd crates/lunch-worker
cp wrangler.example.toml wrangler.toml
npx wrangler login
npx wrangler whoami                          # if several accounts are listed, set account_id in wrangler.toml
npx wrangler d1 create lunch-roulette        # copy the printed database_id into wrangler.toml
npx wrangler d1 migrations apply lunch-roulette --remote
npx wrangler deploy                          # prints https://lunch-roulette.<your-subdomain>.workers.dev
curl https://lunch-roulette.<your-subdomain>.workers.dev/health   # → ok
```

`wrangler.toml` is ignored by git, so your account and database ids stay local.

### Restaurants

The database starts empty. You have two ways to fill it:
- add places from Slack with `/lunch add`;
- load a SQL file. Use `seeds/example.sql` as a template, keep your copy in `seeds/local/` (ignored by git), and run:

```bash
npx wrangler d1 execute lunch-roulette --remote --file ../../seeds/local/my-team.sql
```

### Settings

Settings live in the `settings` table. Each key falls back to its default when the row is missing.

| Key | Default | Meaning |
|---|---|---|
| `office_lat`, `office_lon` | central Stockholm | Office location, used for walking distances. |
| `timezone` | `Europe/Stockholm` | Decides what "today" is and which weekday the opening hours use. |
| `lunch_start`, `lunch_end` | `11:30`, `13:30` | The lunch window; a place must be open for 45 minutes of it. |
| `default_max_walk_min` | `10` | Walking limit for people who have not set their own. |
| `currency` | `SEK` | Currency of dish prices (stored, not displayed yet). |

```bash
npx wrangler d1 execute lunch-roulette --remote --command \
  "INSERT OR REPLACE INTO settings (key, value) VALUES ('office_lat', '52.37'), ('office_lon', '4.89'), ('timezone', 'Europe/Amsterdam')"
```

## 2. Slack

1. `cp slack-manifest.example.yml slack-manifest.yml` and replace `<your-subdomain>` with your workers.dev subdomain.
2. At <https://api.slack.com/apps>, choose **Create New App → From a manifest**, pick your workspace and paste the file. Then **Install to Workspace**.
3. Store the two secrets. Each command prompts for the value; never put secrets in files or command lines.

```bash
npx wrangler secret put SLACK_SIGNING_SECRET   # Basic Information → App Credentials → Signing Secret
npx wrangler secret put SLACK_BOT_TOKEN        # OAuth & Permissions → Bot User OAuth Token (xoxb-…)
```

Try it: run `/lunch` in a channel.
- **Public channels** work without inviting the bot (`chat:write.public`).
- **Private channels** need `/invite @Lunch Roulette` first.

## Updating

```bash
git pull
npx wrangler d1 migrations apply lunch-roulette --remote   # before deploying code that needs them
npx wrangler deploy
```

Back up the database before migrations: `npx wrangler d1 export lunch-roulette --remote --output backup.sql`.

## Troubleshooting

| Symptom | Cause |
|---|---|
| Slack says the command failed | Run `npx wrangler tail` and repeat the command. |
| `401` in `wrangler tail` | Wrong `SLACK_SIGNING_SECRET`, or the request is older than 5 minutes. |
| "I can't post here yet" | Private channel: invite the bot. |
| `worker-build` fails to compile `openssl-sys` | Install the OpenSSL headers (`libssl-dev` on Debian/Ubuntu, `openssl-devel` on Fedora). |
