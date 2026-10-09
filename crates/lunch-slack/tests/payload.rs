use lunch_slack::payload::{Interaction, LunchCommand, parse_interaction, parse_lunch_text, parse_slash};

fn form(payload: &str) -> String {
    serde_urlencoded::to_string([("payload", payload)]).unwrap()
}

#[test]
fn parses_a_slash_command() {
    let body = "token=x&team_id=T1&channel_id=C123&user_id=U456&user_name=anna&command=%2Flunch&text=add\
                &response_url=https%3A%2F%2Fhooks.slack.com%2Fx&trigger_id=1.2.3";
    let cmd = parse_slash(body).unwrap();
    assert_eq!(cmd.channel_id, "C123");
    assert_eq!(cmd.user_id, "U456");
    assert_eq!(cmd.response_url, "https://hooks.slack.com/x");
    assert_eq!(parse_lunch_text(&cmd.text), LunchCommand::Add);
}

#[test]
fn lunch_subcommands() {
    assert_eq!(parse_lunch_text(""), LunchCommand::Open);
    assert_eq!(parse_lunch_text("   "), LunchCommand::Open);
    assert_eq!(parse_lunch_text(" ADD "), LunchCommand::Add);
    assert_eq!(parse_lunch_text("help"), LunchCommand::Help);
    assert_eq!(parse_lunch_text("pizza please"), LunchCommand::Help);
}

#[test]
fn parses_button_clicks() {
    let json = r#"{"type":"block_actions","user":{"id":"U1","username":"anna","name":"anna"},
        "channel":{"id":"C1"},"message":{"ts":"1700.01"},"response_url":"https://hooks.slack.com/a",
        "trigger_id":"t1","actions":[{"action_id":"join","value":"42"}]}"#;
    match parse_interaction(&form(json)).unwrap() {
        Interaction::BlockActions(a) => {
            assert_eq!(a.user.id, "U1");
            assert_eq!(a.user.display(), "anna");
            assert_eq!(a.channel.unwrap().id, "C1");
            assert_eq!(a.message.unwrap().ts, "1700.01");
            assert_eq!(a.actions[0].action_id, "join");
            assert_eq!(a.actions[0].value.as_deref(), Some("42"));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn parses_select_menus() {
    let json = r#"{"type":"block_actions","user":{"id":"U1"},"trigger_id":"t1",
        "actions":[{"action_id":"manual","selected_option":{"value":"42:7"}}]}"#;
    match parse_interaction(&form(json)).unwrap() {
        Interaction::BlockActions(a) => {
            assert_eq!(a.user.display(), "U1");
            assert_eq!(a.actions[0].selected_option.as_ref().unwrap().value, "42:7");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn parses_modal_submissions() {
    let json = r#"{"type":"view_submission","user":{"id":"U1","name":"anna"},
        "view":{"callback_id":"add_restaurant","private_metadata":"C1",
        "state":{"values":{"name":{"v":{"type":"plain_text_input","value":"Curry House"}}}}}}"#;
    match parse_interaction(&form(json)).unwrap() {
        Interaction::ViewSubmission(s) => {
            assert_eq!(s.view.callback_id, "add_restaurant");
            assert_eq!(s.view.private_metadata, "C1");
            assert_eq!(s.view.state.values["name"]["v"].value.as_deref(), Some("Curry House"));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn unknown_interaction_types_are_tolerated() {
    assert!(matches!(
        parse_interaction(&form(r#"{"type":"shortcut","x":1}"#)).unwrap(),
        Interaction::Other
    ));
}

#[test]
fn garbage_is_an_error() {
    assert!(parse_interaction("nope").is_err());
    assert!(parse_interaction(&form("{not json")).is_err());
}
