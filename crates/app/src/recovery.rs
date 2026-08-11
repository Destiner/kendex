use vstack_core::apply;
use vstack_core::env::Env;
use vstack_core::error::CoreError;
use vstack_core::model::Scope;

/// Roll back any apply a crash left half-finished, for every scope this
/// machine knows about — before the first scan, so the UI only ever sees
/// consistent state. Failures are reported, never fatal: a broken journal
/// in one scope must not keep the app from opening.
pub fn recover_on_launch(env: &Env) -> Vec<String> {
    let mut messages = Vec::new();
    let mut scopes = vec![Scope::Global];
    match vstack_core::settings::load(env) {
        Ok(settings) => scopes.extend(
            settings
                .projects
                .into_iter()
                .map(|root| Scope::Project { root }),
        ),
        Err(error) => messages.push(format!(
            "settings unreadable, checking global only: {error}"
        )),
    }
    for scope in scopes {
        match apply::recover_locked(env, &scope) {
            Ok(true) => messages.push(format!("{}: recovered an interrupted apply", scope.label())),
            Ok(false) => {}
            // A live writer holds this scope and recovers it itself.
            Err(CoreError::ScopeBusy { .. }) => {}
            Err(error) => messages.push(format!("{}: recovery failed: {error}", scope.label())),
        }
    }
    messages
}
