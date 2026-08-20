//! The submissions client: submit a repository to the community directory
//! and read back the caller's rows. Every call is bearer-authenticated
//! from the keychain credential; a rejected access token is refreshed
//! once (rotation saved before the retry), and a refresh the server
//! refuses signs this machine out honestly rather than looping.

use serde::Deserialize;

use crate::error::{CoreError, Result};
use crate::registry::credentials::{Credential, CredentialStore};
use crate::registry::login::TokenPair;
use crate::registry::{Fetch, FetchResponse, base_url};

/// What POST /api/v1/submissions answered.
#[derive(Debug, Clone, Deserialize)]
pub struct Submitted {
    pub repo: String,
    /// `pending`, `listed`, `needs-changes`, `delisted`.
    pub status: String,
}

/// One row of GET /api/v1/submissions — what a Mine row polls.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, serde::Serialize, specta::Type)]
pub struct SubmissionRow {
    pub repo: String,
    pub status: String,
    pub status_reason: Option<String>,
    pub head_commit: Option<String>,
    pub indexed_at: Option<String>,
}

#[derive(Deserialize)]
struct WireError {
    error: String,
}

#[derive(Deserialize)]
struct WireSubmissions {
    submissions: Vec<SubmissionRow>,
}

pub fn submit(fetch: &dyn Fetch, store: &dyn CredentialStore, repo: &str) -> Result<Submitted> {
    let body = serde_json::json!({ "repo": repo }).to_string();
    let url = format!("{}/api/v1/submissions", base_url());
    let response = with_access(fetch, store, |access| {
        fetch.post_json_auth(&url, &body, Some(access))
    })?;
    if response.status == 201 {
        return serde_json::from_slice(&response.body).map_err(|error| {
            CoreError::RegistryMalformed {
                why: error.to_string(),
            }
        });
    }
    Err(server_said(&response))
}

pub fn submissions(fetch: &dyn Fetch, store: &dyn CredentialStore) -> Result<Vec<SubmissionRow>> {
    let url = format!("{}/api/v1/submissions", base_url());
    let response = with_access(fetch, store, |access| {
        fetch.get_auth(&url, None, Some(access))
    })?;
    if response.status == 200 {
        let wire: WireSubmissions = serde_json::from_slice(&response.body).map_err(|error| {
            CoreError::RegistryMalformed {
                why: error.to_string(),
            }
        })?;
        return Ok(wire.submissions);
    }
    Err(server_said(&response))
}

/// Whether a usable credential exists at all — the "signed in?" question
/// surfaces ask before offering submit.
pub fn signed_in(store: &dyn CredentialStore) -> Result<bool> {
    Ok(store.load()?.is_some())
}

/// Run one authenticated call, refreshing a rejected access token once.
/// The rotated pair is saved before the retry, so a crash between the two
/// still leaves the newest credential in the keychain.
fn with_access(
    fetch: &dyn Fetch,
    store: &dyn CredentialStore,
    call: impl Fn(&str) -> Result<FetchResponse>,
) -> Result<FetchResponse> {
    let credential = store.load()?.ok_or_else(|| CoreError::Authoring {
        message: "not signed in — run `kendex login` first".to_owned(),
    })?;
    let first = call(&credential.access_token)?;
    if first.status != 401 {
        return Ok(first);
    }
    // Another kendex process may have refreshed meanwhile — a rotation is
    // one-use, so presenting the old refresh token would burn the family.
    // Reload and prefer any newer credential before refreshing ourselves.
    if let Some(newer) = store.load()?
        && newer.access_token != credential.access_token
    {
        let retried = call(&newer.access_token)?;
        if retried.status != 401 {
            return Ok(retried);
        }
    }
    let pair = refresh(fetch, &credential.refresh_token).map_err(|error| {
        // The refresh being refused means this sign-in is over (revoked,
        // expired, or its family burned) — the stale credential is cleared
        // so the next attempt asks for a fresh login instead of retrying a
        // dead one forever. Cleared only while it is still the credential
        // we presented: a concurrent process's fresh rotation must survive.
        let ours = store
            .load()
            .ok()
            .flatten()
            .is_none_or(|kept| kept.refresh_token == credential.refresh_token);
        if ours {
            let _ = store.clear();
        }
        CoreError::Authoring {
            message: format!("your sign-in has expired ({error}) — run `kendex login` again"),
        }
    })?;
    let rotated = Credential {
        endpoint: base_url(),
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
        capabilities: pair.capabilities,
    };
    store.save(&rotated)?;
    let second = call(&rotated.access_token)?;
    if second.status == 401 {
        return Err(CoreError::Authoring {
            message: "the server does not accept this sign-in — run `kendex login` again"
                .to_owned(),
        });
    }
    Ok(second)
}

fn refresh(fetch: &dyn Fetch, refresh_token: &str) -> Result<TokenPair> {
    let body = serde_json::json!({
        "grant_type": "refresh_token",
        "refresh_token": refresh_token,
    })
    .to_string();
    let response = fetch.post_json(&format!("{}/api/v1/device/token", base_url()), &body)?;
    if response.status != 200 {
        return Err(CoreError::RegistryUnavailable {
            why: server_message(&response),
        });
    }
    serde_json::from_slice(&response.body).map_err(|error| CoreError::RegistryMalformed {
        why: error.to_string(),
    })
}

/// The server's own `error` sentence when it wrote one, else the status.
fn server_message(response: &FetchResponse) -> String {
    serde_json::from_slice::<WireError>(&response.body)
        .map(|wire| wire.error)
        .unwrap_or_else(|_| format!("status {}", response.status))
}

fn server_said(response: &FetchResponse) -> CoreError {
    CoreError::Authoring {
        message: server_message(response),
    }
}
