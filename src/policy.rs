//! Policy: a loaded set of rules plus the evaluator that classifies argv
//! vectors against them.
//!
//! v0.1.0 status:
//! - `Policy::from_file` parses Starlark source via `parser::parse_file`.
//! - `Policy::evaluate` is still the stub from 0.0.1 (always Prompt).
//!   The matching algorithm lands in a follow-up ticket.
//! - Load-time `match`/`not_match` self-validation is a follow-up ticket.

use std::path::Path;

use crate::decision::Decision;
use crate::error::Result;
use crate::parser;
use crate::rule::{HostExecutable, PrefixRule};

#[derive(Debug, Default, Clone)]
pub struct Policy {
    rules: Vec<PrefixRule>,
    host_executables: Vec<HostExecutable>,
    source_path: Option<String>,
}

impl Policy {
    /// Load a policy from a `.policy` file.
    ///
    /// The file is parsed as Starlark in `Dialect::Standard`. Each
    /// `prefix_rule()` call produces a `PrefixRule`; each `host_executable()`
    /// call produces a `HostExecutable`. Declaration order is preserved.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path_str = path.as_ref().display().to_string();
        let parsed = parser::parse_file(path)?;
        // TODO(0.1.0): run parser::validate_examples(&parsed) before return.
        Ok(Policy {
            rules: parsed.rules,
            host_executables: parsed.host_executables,
            source_path: Some(path_str),
        })
    }

    /// Parse a policy from an in-memory Starlark source string.
    pub fn from_source(identifier: &str, source: &str) -> Result<Self> {
        let parsed = parser::parse_source(identifier, source)?;
        Ok(Policy {
            rules: parsed.rules,
            host_executables: parsed.host_executables,
            source_path: Some(identifier.to_string()),
        })
    }

    /// All `prefix_rule` declarations, in source order.
    pub fn rules(&self) -> &[PrefixRule] {
        &self.rules
    }

    /// All `host_executable` declarations, in source order.
    pub fn host_executables(&self) -> &[HostExecutable] {
        &self.host_executables
    }

    /// Path the policy was loaded from, or the identifier passed to
    /// `from_source`. `None` on a default-constructed empty policy.
    pub fn source_path(&self) -> Option<&str> {
        self.source_path.as_deref()
    }

    /// Classify a command against this policy.
    ///
    /// Stub for 0.1.0 T1 — always returns `Prompt`. The real matching
    /// algorithm lands in the prefix-matcher ticket.
    pub fn evaluate(&self, _argv: &[String]) -> Decision {
        Decision::Prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_policy_parses_with_expected_counts() {
        let p = Policy::from_file("examples/starter.policy").unwrap();
        assert_eq!(p.rules().len(), 5, "starter.policy has 5 prefix_rule calls");
        assert_eq!(
            p.host_executables().len(),
            1,
            "starter.policy has 1 host_executable"
        );
        assert_eq!(p.source_path(), Some("examples/starter.policy"));
    }

    #[test]
    fn rule_fields_match_source_declarations() {
        let src = r#"
prefix_rule(
    pattern = ["git", "status"],
    decision = "allow",
    justification = "git status is read-only.",
)
"#;
        let p = Policy::from_source("inline", src).unwrap();
        let r = &p.rules()[0];
        assert_eq!(r.pattern, vec!["git".to_string(), "status".to_string()]);
        assert_eq!(r.decision, Decision::Allow);
        assert_eq!(r.justification, "git status is read-only.");
    }

    #[test]
    fn host_executable_fields_match_source() {
        let src = r#"
host_executable(name = "git", paths = ["/usr/bin/git", "/opt/homebrew/bin/git"])
"#;
        let p = Policy::from_source("inline", src).unwrap();
        let he = &p.host_executables()[0];
        assert_eq!(he.name, "git");
        assert_eq!(he.paths, vec!["/usr/bin/git", "/opt/homebrew/bin/git"]);
    }

    #[test]
    fn empty_pattern_is_rejected() {
        let src = r#"prefix_rule(pattern=[], decision="allow", justification="no")"#;
        assert!(Policy::from_source("inline", src).is_err());
    }

    #[test]
    fn invalid_decision_is_rejected() {
        let src = r#"prefix_rule(pattern=["ls"], decision="maybe", justification="no")"#;
        assert!(Policy::from_source("inline", src).is_err());
    }
}
