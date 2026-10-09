//! `/slack/commands` and `/slack/interactions`. Every request is signature-checked and acked
//! within Slack's 3 s; slow work runs in `ctx.wait_until` and reports failures ephemerally.

use std::future::Future;

use lunch_core::model::{PickOptions, PickResponse, User};
use lunch_slack::actions::{ActionValue, LunchAction};
use lunch_slack::blocks;
use lunch_slack::modal;
use lunch_slack::payload::{
    BlockActions, Interaction, LunchCommand, SlashCommand, ViewSubmission, parse_interaction, parse_lunch_text,
    parse_slash,
};
use lunch_slack::verify::verify;
use serde_json::json;
use worker::{Context, Env, Error, Request, Response, Result, console_error, console_warn};

use crate::app;
use crate::slack_client::SlackClient;

/// Reads the raw body and checks Slack's signature. `None` means "answer 401".
async fn verified_body(req: &mut Request, env: &Env) -> Result<Option<String>> {
    let secret = env.secret("SLACK_SIGNING_SECRET")?.to_string();
    let timestamp = req.headers().get("X-Slack-Request-Timestamp")?.unwrap_or_default();
    let signature = req.headers().get("X-Slack-Signature")?.unwrap_or_default();
    let body = req.text().await?;
    match verify(
        &secret,
        &timestamp,
        &body,
        &signature,
        jiff::Timestamp::now().as_second(),
    ) {
        Ok(()) => Ok(Some(body)),
        Err(e) => {
            console_warn!("rejected Slack request: {e}");
            Ok(None)
        }
    }
}

fn bad_request(e: impl std::fmt::Display) -> Error {
    Error::RustError(format!("bad Slack payload: {e}"))
}

/// Runs work after the ack. Failures are logged and, when possible, shown to the user.
async fn background<F, Fut>(env: Env, response_url: Option<String>, work: F)
where
    F: FnOnce(Env) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    if let Err(e) = work(env.clone()).await {
        console_error!("{e}");
        if let (Some(url), Ok(client)) = (response_url, SlackClient::from_env(&env)) {
            let _ = client.respond(&url, &blocks::ephemeral(&format!("😕 {e}"))).await;
        }
    }
}

pub async fn commands(mut req: Request, env: Env, ctx: Context) -> Result<Response> {
    let Some(body) = verified_body(&mut req, &env).await? else {
        return Response::error("invalid signature", 401);
    };
    let cmd = parse_slash(&body).map_err(bad_request)?;
    let url = Some(cmd.response_url.clone());
    match parse_lunch_text(&cmd.text) {
        LunchCommand::Help => Response::from_json(&blocks::ephemeral(blocks::HELP)),
        LunchCommand::Add => {
            ctx.wait_until(background(env, url, move |env| async move {
                open_add_modal(&env, &cmd).await
            }));
            Response::empty()
        }
        LunchCommand::Open => {
            ctx.wait_until(background(
                env,
                url,
                move |env| async move { open_board(&env, &cmd).await },
            ));
            Response::empty()
        }
    }
}

fn fallback_name(cmd: &SlashCommand) -> String {
    if cmd.user_name.trim().is_empty() {
        cmd.user_id.clone()
    } else {
        cmd.user_name.clone()
    }
}

async fn open_board(env: &Env, cmd: &SlashCommand) -> Result<()> {
    let client = SlackClient::from_env(env)?;
    let mut service = app::service(env)?;
    let display_name = client
        .display_name(&cmd.user_id)
        .await
        .unwrap_or_else(|| fallback_name(cmd));
    let actor = User {
        id: cmd.user_id.clone(),
        display_name,
    };
    let view = service
        .open_session(&cmd.channel_id, &actor)
        .await
        .map_err(app::core_err)?;
    let restaurants = service.restaurants().await.map_err(app::core_err)?;

    // Re-running /lunch reposts the board at the bottom; the old copy becomes a pointer.
    if let Some(old_ts) = &view.session.message_ts {
        let moved = json!({"channel": cmd.channel_id, "ts": old_ts, "text": "This lunch board moved further down ⬇️", "blocks": []});
        if let Err(e) = client.call("chat.update", &moved).await {
            console_warn!("could not retire the old board: {e}");
        }
    }
    let mut message = blocks::session_message(&view, &restaurants, None);
    message["channel"] = json!(cmd.channel_id);
    match client.call("chat.postMessage", &message).await {
        Ok(posted) => {
            let ts = posted["ts"]
                .as_str()
                .ok_or_else(|| Error::RustError("chat.postMessage returned no ts".into()))?;
            service.set_message_ts(view.session.id, ts).await.map_err(app::core_err)
        }
        Err(e)
            if ["not_in_channel", "channel_not_found"]
                .iter()
                .any(|code| e.to_string().contains(code)) =>
        {
            let hint = "I can't post here yet — invite me with `/invite @Lunch Roulette` and try again.";
            client.respond(&cmd.response_url, &blocks::ephemeral(hint)).await
        }
        Err(e) => Err(e),
    }
}

async fn open_add_modal(env: &Env, cmd: &SlashCommand) -> Result<()> {
    let client = SlackClient::from_env(env)?;
    let body = json!({"trigger_id": cmd.trigger_id, "view": modal::add_restaurant_modal(&cmd.channel_id)});
    client.call("views.open", &body).await.map(|_| ())
}

pub async fn interactions(mut req: Request, env: Env, ctx: Context) -> Result<Response> {
    let Some(body) = verified_body(&mut req, &env).await? else {
        return Response::error("invalid signature", 401);
    };
    match parse_interaction(&body).map_err(bad_request)? {
        Interaction::BlockActions(actions) => {
            let url = actions.response_url.clone();
            ctx.wait_until(background(env, url, move |env| async move {
                handle_actions(&env, &actions).await
            }));
            Response::empty()
        }
        Interaction::ViewSubmission(submission) if submission.view.callback_id == modal::ADD_RESTAURANT => {
            add_restaurant(env, ctx, submission).await
        }
        _ => Response::empty(),
    }
}

async fn handle_actions(env: &Env, a: &BlockActions) -> Result<()> {
    let client = SlackClient::from_env(env)?;
    let mut service = app::service(env)?;
    for action in &a.actions {
        let Some(kind) = LunchAction::from_id(&action.action_id) else {
            continue;
        };
        let raw = action
            .value
            .clone()
            .or_else(|| action.selected_option.as_ref().map(|o| o.value.clone()));
        let Some(value) = raw.as_deref().and_then(ActionValue::decode) else {
            continue;
        };
        let sid = value.session;
        let actor = User {
            id: a.user.id.clone(),
            display_name: a.user.display(),
        };

        let (view, notice) = match kind {
            LunchAction::Menu => continue,
            LunchAction::Join => {
                let display_name = client
                    .display_name(&actor.id)
                    .await
                    .unwrap_or_else(|| actor.display_name.clone());
                (service.join(sid, &User { display_name, ..actor }).await, None)
            }
            LunchAction::Leave => (service.leave(sid, &actor.id).await, None),
            LunchAction::Pick | LunchAction::IgnoreDistance => {
                let opts = PickOptions {
                    ignore_distance: kind == LunchAction::IgnoreDistance,
                };
                match service.pick(sid, opts).await {
                    Ok(PickResponse::Picked(view)) => (Ok(view), None),
                    Ok(PickResponse::Empty { stage, view }) => (Ok(view), Some(stage)),
                    Err(e) => (Err(e), None),
                }
            }
            LunchAction::Manual => match value.target {
                Some(restaurant) => (service.manual_pick(sid, restaurant).await, None),
                None => continue,
            },
            LunchAction::Accept => match value.target {
                Some(pick) => (service.accept(sid, pick).await, None),
                None => continue,
            },
            LunchAction::Feedback(rating) => {
                let Some(pick) = value.target else { continue };
                service
                    .feedback(sid, pick, &actor, rating)
                    .await
                    .map_err(app::core_err)?;
                if let Some(url) = &a.response_url {
                    client.respond(url, &blocks::ephemeral("Thanks — noted! 🙏")).await?;
                }
                continue;
            }
        };
        let view = view.map_err(app::core_err)?;
        let restaurants = service.restaurants().await.map_err(app::core_err)?;
        // Update the message that was clicked, even if a newer board exists.
        let channel = a
            .channel
            .as_ref()
            .map_or_else(|| view.session.channel_id.clone(), |c| c.id.clone());
        let Some(ts) = a
            .message
            .as_ref()
            .map(|m| m.ts.clone())
            .or_else(|| view.session.message_ts.clone())
        else {
            continue;
        };
        let mut message = blocks::session_message(&view, &restaurants, notice);
        message["channel"] = json!(channel);
        message["ts"] = json!(ts);
        client.call("chat.update", &message).await?;
    }
    Ok(())
}

/// Modal submissions must be answered synchronously: errors keep the modal open.
async fn add_restaurant(env: Env, ctx: Context, submission: ViewSubmission) -> Result<Response> {
    let input = match modal::parse_add_restaurant(&submission.view) {
        Ok(input) => input,
        Err(errors) => return Response::from_json(&modal::errors_response(&errors)),
    };
    let mut service = app::service(&env)?;
    match service.add_restaurant(input.clone()).await {
        Ok(_) => {
            let channel = submission.view.private_metadata.clone();
            let user = submission.user.id.clone();
            ctx.wait_until(background(env, None, move |env| async move {
                let client = SlackClient::from_env(&env)?;
                let text = format!("✅ Added *{}* to the lunch list.", blocks::escape(&input.name));
                client
                    .call(
                        "chat.postEphemeral",
                        &json!({"channel": channel, "user": user, "text": text}),
                    )
                    .await
                    .map(|_| ())
            }));
            Response::empty()
        }
        Err(lunch_core::Error::Invalid(message)) => {
            Response::from_json(&modal::errors_response(&[("name".to_string(), message)].into()))
        }
        Err(e) => Err(app::core_err(e)),
    }
}
