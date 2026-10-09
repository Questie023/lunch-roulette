use lunch_slack::verify::{VerifyError, sign, verify};

// The worked example from https://docs.slack.dev/authentication/verifying-requests-from-slack
const SECRET: &str = "8f742231b10e8888abcd99yyyzzz85a5";
const TS: &str = "1531420618";
const BODY: &str = "token=xyzz0WbapA4vBCDEFasx0q6G&team_id=T1DC2JH3J&team_domain=testteamnow&channel_id=G8PSS9T3V&channel_name=foobar&user_id=U2CERLKJA&user_name=roadrunner&command=%2Fwebhook-collect&text=&response_url=https%3A%2F%2Fhooks.slack.com%2Fcommands%2FT1DC2JH3J%2F397700885554%2F96rGlfmibIGlgcZRskXaIFfN&trigger_id=398738663015.47445629121.803a0bc887a14d10d2c447fce8b6703c";
const SIG: &str = "v0=a2114d57b48eac39b9ad189dd8316235a7b4a8d21a10bd27519666489c69b503";
const NOW: i64 = 1_531_420_618;

#[test]
fn slack_docs_example_verifies() {
    assert_eq!(verify(SECRET, TS, BODY, SIG, NOW), Ok(()));
}

#[test]
fn sign_matches_slack() {
    assert_eq!(sign(SECRET, TS, BODY), SIG);
}

#[test]
fn stale_or_future_requests_are_rejected() {
    assert_eq!(verify(SECRET, TS, BODY, SIG, NOW + 301), Err(VerifyError::Stale));
    assert_eq!(verify(SECRET, TS, BODY, SIG, NOW - 301), Err(VerifyError::Stale));
}

#[test]
fn tampered_body_or_wrong_secret_is_rejected() {
    let tampered = BODY.replace("roadrunner", "coyote");
    assert_eq!(verify(SECRET, TS, &tampered, SIG, NOW), Err(VerifyError::Mismatch));
    assert_eq!(verify("other-secret", TS, BODY, SIG, NOW), Err(VerifyError::Mismatch));
}

#[test]
fn malformed_headers_are_rejected() {
    assert_eq!(verify(SECRET, "abc", BODY, SIG, NOW), Err(VerifyError::BadTimestamp));
    assert_eq!(
        verify(SECRET, TS, BODY, &SIG[3..], NOW),
        Err(VerifyError::BadSignatureFormat)
    );
    assert_eq!(
        verify(SECRET, TS, BODY, "v0=zz", NOW),
        Err(VerifyError::BadSignatureFormat)
    );
    assert_eq!(verify(SECRET, TS, BODY, "", NOW), Err(VerifyError::BadSignatureFormat));
}
