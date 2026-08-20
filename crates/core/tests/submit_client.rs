//! The submissions client: bearer auth from the stored credential, one
//! refresh on a rejected access token (rotation saved before the retry),
//! and an honest sign-out when the refresh itself is refused.

use std::cell::RefCell;

use kendex_core::error::Result;
use kendex_core::registry::credentials::{Credential, CredentialStore};
use kendex_core::registry::submit::{submissions, submit};
use kendex_core::registry::{Fetch, FetchResponse};

struct Canned {
    answers: RefCell<Vec<(u16, String)>>,
    bearers: RefCell<Vec<Option<String>>>,
}

impl Canned {
    fn new(answers: Vec<(u16, &str)>) -> Canned {
        Canned {
            answers: RefCell::new(
                answers
                    .into_iter()
                    .map(|(status, body)| (status, body.to_owned()))
                    .collect(),
            ),
            bearers: RefCell::new(Vec::new()),
        }
    }

    fn next(&self, bearer: Option<&str>) -> Result<FetchResponse> {
        self.bearers.borrow_mut().push(bearer.map(str::to_owned));
        let (status, body) = self.answers.borrow_mut().remove(0);
        Ok(FetchResponse {
            status,
            etag: None,
            body: body.into_bytes(),
        })
    }
}

impl Fetch for Canned {
    fn get_auth(
        &self,
        _url: &str,
        _etag: Option<&str>,
        bearer: Option<&str>,
    ) -> Result<FetchResponse> {
        self.next(bearer)
    }

    fn post_json_auth(
        &self,
        _url: &str,
        _body: &str,
        bearer: Option<&str>,
    ) -> Result<FetchResponse> {
        self.next(bearer)
    }
}

struct MemoryStore(RefCell<Option<Credential>>);

impl MemoryStore {
    fn signed_in() -> MemoryStore {
        MemoryStore(RefCell::new(Some(Credential {
            endpoint: "https://kendex.ai".to_owned(),
            access_token: "kxa_old".to_owned(),
            refresh_token: "kxr_old".to_owned(),
            capabilities: vec!["submission:write".to_owned()],
        })))
    }
}

impl CredentialStore for MemoryStore {
    fn save(&self, credential: &Credential) -> Result<()> {
        *self.0.borrow_mut() = Some(credential.clone());
        Ok(())
    }
    fn load(&self) -> Result<Option<Credential>> {
        Ok(self.0.borrow().clone())
    }
    fn clear(&self) -> Result<()> {
        *self.0.borrow_mut() = None;
        Ok(())
    }
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_fresh_credential_submits_in_one_call() {
    let fetch = Canned::new(vec![(
        201,
        r#"{"ok":true,"repo":"jane/skills","status":"pending"}"#,
    )]);
    let store = MemoryStore::signed_in();
    let outcome = submit(&fetch, &store, "jane/skills").unwrap();
    assert_eq!(outcome.status, "pending");
    assert_eq!(
        fetch.bearers.borrow().as_slice(),
        [Some("kxa_old".to_owned())]
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_rejected_access_token_refreshes_once_and_saves_the_rotation() {
    let fetch = Canned::new(vec![
        (401, r#"{"error":"invalid_token"}"#),
        (
            200,
            r#"{"access_token":"kxa_new","refresh_token":"kxr_new","capabilities":["submission:write"]}"#,
        ),
        (201, r#"{"ok":true,"repo":"jane/skills","status":"listed"}"#),
    ]);
    let store = MemoryStore::signed_in();
    let outcome = submit(&fetch, &store, "jane/skills").unwrap();
    assert_eq!(outcome.status, "listed");
    // The rotated pair replaced the old one before the retry ran.
    let kept = store.load().unwrap().unwrap();
    assert_eq!(kept.access_token, "kxa_new");
    assert_eq!(kept.refresh_token, "kxr_new");
    let bearers = fetch.bearers.borrow();
    assert_eq!(bearers[0].as_deref(), Some("kxa_old"));
    assert_eq!(
        bearers[1], None,
        "the refresh call itself is not bearer-authenticated"
    );
    assert_eq!(bearers[2].as_deref(), Some("kxa_new"));
}

#[test]
#[allow(clippy::unwrap_used)]
fn a_refused_refresh_signs_this_machine_out() {
    let fetch = Canned::new(vec![
        (401, r#"{"error":"invalid_token"}"#),
        (401, r#"{"error":"invalid_grant"}"#),
    ]);
    let store = MemoryStore::signed_in();
    let refused = submit(&fetch, &store, "jane/skills")
        .unwrap_err()
        .to_string();
    assert!(refused.contains("run `kendex login` again"), "{refused}");
    assert!(
        store.load().unwrap().is_none(),
        "a dead credential must not be kept for endless retries"
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn signed_out_asks_for_login_before_any_network_call() {
    let fetch = Canned::new(vec![]);
    let store = MemoryStore(RefCell::new(None));
    let refused = submit(&fetch, &store, "jane/skills")
        .unwrap_err()
        .to_string();
    assert!(refused.contains("not signed in"), "{refused}");
    assert!(fetch.bearers.borrow().is_empty());
}

#[test]
#[allow(clippy::unwrap_used)]
fn the_server_refusal_sentence_reaches_the_caller_verbatim() {
    let fetch = Canned::new(vec![(
        403,
        r#"{"error":"you do not hold push authority over this repository"}"#,
    )]);
    let store = MemoryStore::signed_in();
    let refused = submit(&fetch, &store, "jane/skills")
        .unwrap_err()
        .to_string();
    assert!(refused.contains("push authority"), "{refused}");
}

#[test]
#[allow(clippy::unwrap_used)]
fn submissions_parse_the_versioned_rows() {
    let fetch = Canned::new(vec![(
        200,
        r#"{"schema":1,"submissions":[{"repo":"jane/skills","status":"needs-changes","status_reason":"description missing","head_commit":null,"indexed_at":null}]}"#,
    )]);
    let store = MemoryStore::signed_in();
    let rows = submissions(&fetch, &store).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, "needs-changes");
    assert_eq!(
        rows[0].status_reason.as_deref(),
        Some("description missing")
    );
}
