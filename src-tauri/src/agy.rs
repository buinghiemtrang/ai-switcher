//! Read-only integration with Google's Antigravity CLI. Credentials stay in
//! SecretService and are never refreshed, copied into app state, or logged.
use anyhow::{Context, Result};
use crate::models::{QuotaInfo, QuotaWindow};
use std::path::PathBuf;

pub(crate) fn binary() -> Option<PathBuf> {
    if cfg!(target_os = "linux") { crate::tools::command_path("agy") } else { None }
}

pub(crate) fn config_dir() -> PathBuf {
    crate::tools::home_dir().join(".gemini/antigravity-cli")
}

struct Credentials {
    access_token: String,
    identity: String,
}

fn parse_credentials(bytes: &[u8]) -> Result<Credentials> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| anyhow::anyhow!("Antigravity CLI credentials could not be decoded"))?;
    let token = value.get("token").context("Antigravity CLI has no OAuth token; sign in using agy")?;
    let access_token = token.get("access_token").and_then(|v| v.as_str())
        .filter(|v| !v.is_empty()).context("Antigravity CLI has no access token; sign in using agy")?.to_owned();
    let expiry = token.get("expiry").and_then(|v| v.as_str())
        .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
        .context("Antigravity CLI token expiry is unknown; open agy to renew it")?;
    if expiry.timestamp() <= chrono::Utc::now().timestamp() + 30 {
        anyhow::bail!("Antigravity CLI OAuth token expired; open agy to renew the session, then Refresh. AI switcher does not refresh credentials.");
    }
    let identity = value.get("id_token_email").and_then(|v| v.as_str())
        .filter(|v| !v.is_empty()).context("Antigravity CLI account identity is unavailable")?.to_owned();
    Ok(Credentials { access_token, identity })
}

#[cfg(target_os = "linux")]
fn credentials() -> Result<Credentials> {
    use dbus::{blocking::Connection, arg::{RefArg, Variant}, Path};
    use std::{collections::HashMap, time::Duration};
    let bus = Connection::new_session().map_err(|_| anyhow::anyhow!("Cannot connect to the desktop credential store"))?;
    let service = bus.with_proxy("org.freedesktop.secrets", "/org/freedesktop/secrets", Duration::from_secs(3));
    let attributes: HashMap<&str, &str> = [("service", "gemini"), ("username", "antigravity")].into_iter().collect();
    let (unlocked, locked): (Vec<Path<'static>>, Vec<Path<'static>>) = service.method_call("org.freedesktop.Secret.Service", "SearchItems", (attributes,))
        .map_err(|_| anyhow::anyhow!("Cannot search Antigravity CLI credentials in the desktop store"))?;
    if unlocked.is_empty() && !locked.is_empty() { anyhow::bail!("Antigravity CLI credentials are locked; unlock your desktop keyring and Refresh"); }
    if unlocked.len() + locked.len() != 1 || unlocked.len() != 1 { anyhow::bail!("Antigravity CLI login is missing or ambiguous; open agy and sign in"); }
    let (_, session): (Variant<Box<dyn RefArg>>, Path<'static>) = service.method_call("org.freedesktop.Secret.Service", "OpenSession", ("plain", Variant(String::new())))
        .map_err(|_| anyhow::anyhow!("Cannot open a read-only desktop credential session"))?;
    let item = bus.with_proxy("org.freedesktop.secrets", unlocked[0].clone(), Duration::from_secs(3));
    type Secret = (Path<'static>, Vec<u8>, Vec<u8>, String);
    let result: std::result::Result<(Secret,), _> = item.method_call("org.freedesktop.Secret.Item", "GetSecret", (session.clone(),));
    let _: std::result::Result<(), _> = bus.with_proxy("org.freedesktop.secrets", session, Duration::from_secs(3)).method_call("org.freedesktop.Secret.Session", "Close", ());
    let ((_, _, bytes, _),) = result.map_err(|_| anyhow::anyhow!("Cannot read Antigravity CLI credentials; unlock your desktop keyring"))?;
    parse_credentials(&bytes)
}

#[cfg(not(target_os = "linux"))]
fn credentials() -> Result<Credentials> {
    anyhow::bail!("Antigravity CLI quota integration currently requires Linux SecretService")
}

pub(crate) fn read_quota() -> Result<QuotaInfo> {
    let credential = credentials()?;
    let quota = fetch_quota(&credential.access_token)?;
    // An external login during the request must not put one identity's quota on
    // the machine-default row for a different identity.
    if credentials()?.identity != credential.identity {
        anyhow::bail!("Antigravity CLI account changed while reading quota; Refresh again");
    }
    Ok(quota)
}

fn fetch_quota(token: &str) -> Result<QuotaInfo> {
    let client = reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(15)).build()?;
    let load = agy_post(&client, token, "loadCodeAssist", &serde_json::json!({}))?;
    let project = load.get("cloudaicompanionProject").and_then(|v| v.as_str())
        .filter(|v| !v.is_empty()).context("Antigravity CLI account has no quota project; open agy to complete setup")?;
    let models = agy_post(&client, token, "fetchAvailableModels", &serde_json::json!({"project": project}))?;
    let mut quota = parse_model_quota(&models)?;
    quota.plan = load.get("paidTier").or_else(|| load.get("currentTier"))
        .and_then(|tier| tier.get("name").or_else(|| tier.get("id")))
        .and_then(|v| v.as_str()).filter(|v| !v.is_empty()).map(str::to_owned);
    Ok(quota)
}

fn agy_post(client: &reqwest::blocking::Client, token: &str, method: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
    let response = client.post(format!("https://cloudcode-pa.googleapis.com/v1internal:{method}"))
        .bearer_auth(token).json(body).send()
        .map_err(|_| anyhow::anyhow!("Antigravity CLI quota network request failed"))?;
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("Antigravity CLI quota API returned HTTP {}. Open agy to check your session, then Refresh. AI switcher does not refresh credentials.", status.as_u16());
    }
    response.json().map_err(|_| anyhow::anyhow!("Antigravity CLI quota API returned an invalid response"))
}

fn parse_model_quota(value: &serde_json::Value) -> Result<QuotaInfo> {
    let entries = value.get("models").and_then(|v| v.as_object())
        .context("Antigravity CLI API did not report per-model quota")?;
    let mut windows = Vec::new();
    for (id, model) in entries {
        let Some(info) = model.get("quotaInfo") else { continue; };
        let reset_at = info.get("resetTime").and_then(|v| v.as_str())
            .filter(|v| chrono::DateTime::parse_from_rfc3339(v).is_ok()).map(str::to_owned);
        // Observed protobuf JSON omits remainingFraction when it is zero.
        // A reset-only quota is exhausted; an empty or malformed object is unknown.
        let remaining = match info.get("remainingFraction") {
            Some(value) => match value.as_f64() { Some(value) => value, None => continue },
            None if reset_at.is_some() => 0.0,
            None => continue,
        };
        if !remaining.is_finite() || !(0.0..=1.0).contains(&remaining) { continue; }
        windows.push(QuotaWindow {
            label: model.get("displayName").and_then(|v| v.as_str()).unwrap_or(id).to_owned(),
            percent_used: Some((1.0 - remaining) * 100.0),
            reset_at,
            is_active: None,
        });
    }
    let worst = windows.iter().max_by(|a, b| a.percent_used.unwrap_or_default().total_cmp(&b.percent_used.unwrap_or_default()))
        .context("Antigravity CLI API did not expose usable model quota; no usage percentage is available")?;
    let mut quota = QuotaInfo::with_message("");
    quota.error = None;
    quota.five_hour = QuotaWindow { label: "Model quota".to_owned(), ..worst.clone() };
    quota.models = Some(windows);
    Ok(quota)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_model_quota_is_unknown_not_zero() {
        assert!(parse_model_quota(&serde_json::json!({"models":{"m":{"quotaInfo":{}}}})).is_err());
        assert!(parse_model_quota(&serde_json::json!({"models":{"m":{"quotaInfo":{"remainingFraction":2}}}})).is_err());
    }
    #[test]
    fn reset_only_model_quota_is_exhausted_protobuf_default() {
        let quota = parse_model_quota(&serde_json::json!({"models":{"m":{"quotaInfo":{"resetTime":"2099-01-01T00:00:00Z"}}}})).unwrap();
        assert_eq!(quota.five_hour.percent_used, Some(100.0));
        assert!(parse_model_quota(&serde_json::json!({"models":{"m":{"quotaInfo":{"resetTime":"invalid"}}}})).is_err());
    }
    #[test]
    fn model_quota_is_parsed_only_when_explicit() {
        let quota = parse_model_quota(&serde_json::json!({"models":{"m":{"displayName":"Fixture model","quotaInfo":{"remainingFraction":0.25,"resetTime":"2099-01-01T00:00:00Z"}}, "unknown":{}}})).unwrap();
        assert_eq!(quota.five_hour.percent_used, Some(75.0));
        assert_eq!(quota.models.unwrap().len(), 1);
    }
    #[test]
    fn expired_cli_credentials_are_not_refreshed_or_accepted() {
        let value = serde_json::json!({"token":{"access_token":"synthetic", "expiry":"2000-01-01T00:00:00Z"}, "id_token_email":"fixture@example.invalid"});
        assert!(parse_credentials(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    #[test]
    fn cli_credentials_require_identity_and_expiry() {
        assert!(parse_credentials(br#"{"token":{"access_token":"synthetic"}}"#).is_err());
        let value = serde_json::json!({"token":{"access_token":"synthetic", "expiry":"2099-01-01T00:00:00Z"}, "id_token_email":"fixture@example.invalid"});
        assert_eq!(parse_credentials(&serde_json::to_vec(&value).unwrap()).unwrap().identity, "fixture@example.invalid");
    }
}
