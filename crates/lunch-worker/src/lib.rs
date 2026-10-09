//! Cloudflare Worker entry point: routes Slack requests into `LunchService` over D1.

mod app;
mod d1;
mod handlers;
mod slack_client;

use worker::{Context, Env, Method, Request, Response, Result, event};

#[event(fetch)]
async fn fetch(req: Request, env: Env, ctx: Context) -> Result<Response> {
    let path = req.path();
    match (req.method(), path.as_str()) {
        (Method::Get, "/health") => Response::ok("ok"),
        (Method::Post, "/slack/commands") => handlers::commands(req, env, ctx).await,
        (Method::Post, "/slack/interactions") => handlers::interactions(req, env, ctx).await,
        (Method::Post, "/__selftest") if app::flag(&env, "SELFTEST") => app::selftest(&env).await,
        _ => Response::error("not found", 404),
    }
}
