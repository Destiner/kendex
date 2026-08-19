//! `kendex login` and `kendex logout`: the device flow from the terminal
//! side. Sign-in needs no password here — a code, a browser tab, done —
//! and the credential lives in the OS keychain or nowhere.

use super::say;
use kendex_core::error::{CoreError, Result};
use kendex_core::registry::credentials::{Credential, CredentialStore, KeyringStore};
use kendex_core::registry::login::{self, Poll};
use kendex_core::registry::{CurlFetch, base_url};

pub fn login() -> Result<()> {
    let fetch = CurlFetch;
    let store = KeyringStore;
    if let Ok(Some(_)) = store.load() {
        say(&format!(
            "Already signed in to {} — run `kendex logout` first to switch.",
            base_url()
        ));
        return Ok(());
    }
    let started = login::start(&fetch)?;
    say(&format!(
        "First, open:  {}?code={}",
        started.verification_url, started.user_code
    ));
    say(&format!("Your code:    {}", started.user_code));
    say("");
    say(&format!(
        "Waiting for approval… (expires in {} minutes)",
        started.expires_in_seconds / 60
    ));

    let mut interval = started.interval_seconds;
    loop {
        std::thread::sleep(std::time::Duration::from_secs(interval));
        match login::poll_once(&fetch, &started.device_code)? {
            Poll::Pending => {}
            Poll::SlowDown => interval += 5,
            Poll::Signed(pair) => {
                store.save(&Credential {
                    endpoint: base_url(),
                    access_token: pair.access_token,
                    refresh_token: pair.refresh_token,
                    capabilities: pair.capabilities,
                })?;
                say("Signed in. The credential is in your system keychain.");
                return Ok(());
            }
        }
    }
}

pub fn logout() -> Result<()> {
    let store = KeyringStore;
    let Some(credential) = store.load()? else {
        say("Not signed in.");
        return Ok(());
    };
    // Server first: if revocation cannot be recorded, the local copy
    // stays so a retry still has something to revoke.
    login::revoke(&CurlFetch, &credential.refresh_token).map_err(|error| {
        CoreError::RegistryUnavailable {
            why: format!("{error} — the local credential was kept so you can retry"),
        }
    })?;
    store.clear()?;
    say("Signed out — every device credential in that sign-in is now dead.");
    Ok(())
}
