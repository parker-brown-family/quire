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
        parser::validate_examples(&parsed.rules, &parsed.pending_validations)?;
        Ok(Policy {
            rules: parsed.rules,
            host_executables: parsed.host_executables,
            source_path: Some(path_str),
        })
    }

    /// Parse a policy from an in-memory Starlark source string.
    pub fn from_source(identifier: &str, source: &str) -> Result<Self> {
        let parsed = parser::parse_source(identifier, source)?;
        parser::validate_examples(&parsed.rules, &parsed.pending_validations)?;
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
    /// Scans `self.rules()` in source-declaration order and returns the
    /// `decision` of the first rule whose `pattern` is a token-prefix of
    /// `argv`. Returns `Decision::NoMatch` if no rule matches — harnesses
    /// should treat that as "fall back to your own permission prompt."
    pub fn evaluate(&self, argv: &[String]) -> Decision {
        self.rules
            .iter()
            .find(|r| r.matches(argv))
            .map(|r| r.decision)
            .unwrap_or(Decision::NoMatch)
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

    fn argv(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|s| s.to_string()).collect()
    }

    fn starter() -> Policy {
        Policy::from_file("examples/starter.policy").unwrap()
    }

    #[test]
    fn default_empty_policy_returns_no_match() {
        let p = Policy::default();
        assert_eq!(p.evaluate(&argv(&["ls"])), Decision::NoMatch);
    }

    #[test]
    fn starter_policy_allows_ls() {
        assert_eq!(starter().evaluate(&argv(&["ls", "-la"])), Decision::Allow);
    }

    #[test]
    fn starter_policy_allows_cat() {
        assert_eq!(
            starter().evaluate(&argv(&["cat", "README.md"])),
            Decision::Allow
        );
    }

    #[test]
    fn starter_policy_prompts_git_push() {
        assert_eq!(
            starter().evaluate(&argv(&["git", "push", "origin", "main"])),
            Decision::Prompt
        );
    }

    #[test]
    fn starter_policy_forbids_rm_rf() {
        assert_eq!(
            starter().evaluate(&argv(&["rm", "-rf", "/"])),
            Decision::Forbidden
        );
    }

    #[test]
    fn starter_policy_no_match_on_unknown() {
        assert_eq!(
            starter().evaluate(&argv(&["wget", "http://x"])),
            Decision::NoMatch
        );
    }

    #[test]
    fn starter_policy_rejects_prefix_lookalike() {
        // `lsof` starts with "ls" as a string but is a different program;
        // prefix matching is on whole tokens, not substrings.
        assert_eq!(starter().evaluate(&argv(&["lsof"])), Decision::NoMatch);
    }

    #[test]
    fn first_matching_rule_wins_in_source_order() {
        let src = r#"
prefix_rule(pattern=["git"], decision="prompt",
            justification="any git command needs confirmation")
prefix_rule(pattern=["git", "status"], decision="allow",
            justification="status is read-only")
"#;
        let p = Policy::from_source("inline", src).unwrap();
        // Even though ["git", "status"] matches the second rule too, the
        // first-in-file rule wins.
        assert_eq!(p.evaluate(&argv(&["git", "status"])), Decision::Prompt);
    }

    #[test]
    fn longer_patterns_still_match_their_argv() {
        let src = r#"
prefix_rule(pattern=["git", "push", "--force"], decision="forbidden",
            justification="never force-push")
"#;
        let p = Policy::from_source("inline", src).unwrap();
        assert_eq!(
            p.evaluate(&argv(&["git", "push", "--force", "origin"])),
            Decision::Forbidden
        );
        assert_eq!(
            p.evaluate(&argv(&["git", "push", "origin"])),
            Decision::NoMatch
        );
    }

    // ---- T3: load-time match / not_match self-validation ------------------

    #[test]
    fn match_example_that_does_not_match_its_rule_fails_to_load() {
        // "lsof" does not start with the token "ls", so claiming match=["lsof"]
        // for a rule patterned on ["ls"] is a lie. The load must reject it.
        let src = r#"
prefix_rule(pattern=["ls"], decision="allow",
            justification="ls is read-only",
            match=["lsof"])
"#;
        let err = Policy::from_source("inline", src).unwrap_err().to_string();
        assert!(
            err.contains("match=") && err.contains("lsof"),
            "error should mention the bad example: {err}"
        );
    }

    #[test]
    fn match_example_that_matches_a_different_rule_fails_to_load() {
        // `git status` is claimed as a match of the `git` rule, but the
        // more-specific `git status` rule is declared first and takes it.
        let src = r#"
prefix_rule(pattern=["git", "status"], decision="allow",
            justification="status is read-only")
prefix_rule(pattern=["git"], decision="prompt",
            justification="any git command needs review",
            match=["git status"])
"#;
        let err = Policy::from_source("inline", src).unwrap_err().to_string();
        assert!(
            err.contains("git status") && err.contains("did not match"),
            "error should name the example and the mismatch: {err}"
        );
    }

    #[test]
    fn not_match_example_that_does_match_fails_to_load() {
        // "ls -la" obviously matches pattern=["ls"]; declaring it as
        // not_match is a lie.
        let src = r#"
prefix_rule(pattern=["ls"], decision="allow",
            justification="ls is read-only",
            not_match=["ls -la"])
"#;
        let err = Policy::from_source("inline", src).unwrap_err().to_string();
        assert!(
            err.contains("not_match") && err.contains("ls -la"),
            "error should name the bad not_match example: {err}"
        );
    }

    #[test]
    fn not_match_example_matching_a_different_rule_is_fine() {
        // "wget" does not match our rule but might be thought of as "shouldnt
        // match" — not_match is only about THIS rule, so this loads cleanly.
        let src = r#"
prefix_rule(pattern=["ls"], decision="allow",
            justification="ls is read-only",
            not_match=["wget"])
"#;
        assert!(Policy::from_source("inline", src).is_ok());
    }

    #[test]
    fn valid_match_and_not_match_examples_load_cleanly() {
        let src = r#"
prefix_rule(pattern=["ls"], decision="allow",
            justification="ls is read-only",
            match=["ls", "ls -la", "ls /tmp"],
            not_match=["lsof", "lsblk"])
"#;
        let p = Policy::from_source("inline", src).unwrap();
        assert_eq!(p.rules().len(), 1);
    }
}
