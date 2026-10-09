//! Slack request signing, v0: `v0=` + hex(HMAC-SHA256(secret, "v0:{timestamp}:{raw body}")).

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// Requests older (or newer) than this are rejected to stop replays.
pub const MAX_SKEW_SECS: i64 = 300;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VerifyError {
    #[error("missing or malformed timestamp")]
    BadTimestamp,
    #[error("request timestamp is outside the allowed window")]
    Stale,
    #[error("missing or malformed signature")]
    BadSignatureFormat,
    #[error("signature mismatch")]
    Mismatch,
}

fn mac(secret: &str, timestamp: &str, body: &str) -> Hmac<Sha256> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts keys of any length");
    mac.update(b"v0:");
    mac.update(timestamp.as_bytes());
    mac.update(b":");
    mac.update(body.as_bytes());
    mac
}

pub fn verify(secret: &str, timestamp: &str, body: &str, signature: &str, now_unix: i64) -> Result<(), VerifyError> {
    let ts: i64 = timestamp.parse().map_err(|_| VerifyError::BadTimestamp)?;
    if (now_unix - ts).abs() > MAX_SKEW_SECS {
        return Err(VerifyError::Stale);
    }
    let hex_sig = signature.strip_prefix("v0=").ok_or(VerifyError::BadSignatureFormat)?;
    let expected = hex::decode(hex_sig).map_err(|_| VerifyError::BadSignatureFormat)?;
    // `verify_slice` compares in constant time.
    mac(secret, timestamp, body)
        .verify_slice(&expected)
        .map_err(|_| VerifyError::Mismatch)
}

/// Produces the header value Slack would send. Used by tests and `tools/slack-sim`.
pub fn sign(secret: &str, timestamp: &str, body: &str) -> String {
    format!(
        "v0={}",
        hex::encode(mac(secret, timestamp, body).finalize().into_bytes())
    )
}
