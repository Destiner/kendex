use std::fmt;

/// v1 scope-flag semantics: `--scope` beats `--global`, `--global` alone
/// means global, otherwise the per-command default applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeFilter {
    Project,
    Global,
    All,
}

impl fmt::Display for ScopeFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ScopeFilter::Project => "project",
            ScopeFilter::Global => "global",
            ScopeFilter::All => "all",
        })
    }
}

impl ScopeFilter {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "project" | "p" | "local" => Ok(ScopeFilter::Project),
            "global" | "g" | "user" => Ok(ScopeFilter::Global),
            "all" | "both" | "*" => Ok(ScopeFilter::All),
            other => Err(format!(
                "unknown scope '{other}' (expected project, global, or all)"
            )),
        }
    }

    pub fn resolve(
        scope: Option<&str>,
        global: bool,
        default: ScopeFilter,
    ) -> Result<Self, String> {
        match scope {
            Some(value) => Self::parse(value),
            None if global => Ok(ScopeFilter::Global),
            None => Ok(default),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_beats_global_and_aliases_parse() {
        assert_eq!(
            ScopeFilter::resolve(Some("project"), true, ScopeFilter::All).unwrap(),
            ScopeFilter::Project
        );
        assert_eq!(
            ScopeFilter::resolve(None, true, ScopeFilter::All).unwrap(),
            ScopeFilter::Global
        );
        assert_eq!(
            ScopeFilter::resolve(None, false, ScopeFilter::All).unwrap(),
            ScopeFilter::All
        );
        for (alias, want) in [
            ("p", ScopeFilter::Project),
            ("local", ScopeFilter::Project),
            ("user", ScopeFilter::Global),
            ("both", ScopeFilter::All),
            ("*", ScopeFilter::All),
        ] {
            assert_eq!(ScopeFilter::parse(alias).unwrap(), want);
        }
        assert!(ScopeFilter::parse("everywhere").is_err());
    }
}
