//! Rule types produced by the Starlark policy loader.
//!
//! v0.1.0 owns the data shape plus the single-rule `matches` predicate.
//! Policy-level orchestration (iterate rules, return first match) lives in
//! `policy.rs`. Load-time `match`/`not_match` invariant checks live in the
//! follow-up validation ticket.

use serde::{Deserialize, Serialize};

use crate::decision::Decision;

/// A rule that matches commands whose argv starts with `pattern`.
///
/// The `match` and `not_match` example lists declared in the source are
/// NOT stored on the runtime rule. They are load-time invariants, validated
/// once in `Policy::from_file` and then discarded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrefixRule {
    /// argv-token prefix this rule matches. Non-empty (enforced at load).
    pub pattern: Vec<String>,
    /// Decision to return when this rule matches.
    pub decision: Decision,
    /// Human-readable reason the rule exists. Surfaced in prompt / reject
    /// messages by the calling harness.
    pub justification: String,
}

impl PrefixRule {
    /// Is `argv` token-prefixed by this rule's `pattern`?
    ///
    /// Tokens must match exactly — no glob, no regex, no case folding. A
    /// rule with pattern `["git", "status"]` matches argv `["git", "status"]`
    /// and `["git", "status", "--short"]`, but not `["git", "stash"]`,
    /// `["git", "status-reset"]`, or `["mygit", "status"]`.
    pub fn matches(&self, argv: &[String]) -> bool {
        if argv.len() < self.pattern.len() || self.pattern.is_empty() {
            return false;
        }
        self.pattern
            .iter()
            .zip(argv.iter())
            .all(|(pat, arg)| pat == arg)
    }
}

/// Declares an executable that rules may reference by basename.
///
/// v0.1.0 parses these into the Policy but does not yet consult them during
/// evaluation — that ships in a later release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostExecutable {
    pub name: String,
    pub paths: Vec<String>,
}

/// Union type so future rule shapes can join without breaking the Policy
/// schema. v0 only has PrefixRule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Rule {
    Prefix(PrefixRule),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(pattern: &[&str]) -> PrefixRule {
        PrefixRule {
            pattern: pattern.iter().map(|s| s.to_string()).collect(),
            decision: Decision::Allow,
            justification: "test".into(),
        }
    }

    fn argv(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn single_token_pattern_matches_exact_and_longer_argv() {
        let r = rule(&["ls"]);
        assert!(r.matches(&argv(&["ls"])));
        assert!(r.matches(&argv(&["ls", "-la"])));
    }

    #[test]
    fn single_token_pattern_does_not_match_different_program() {
        let r = rule(&["ls"]);
        assert!(!r.matches(&argv(&["lsof"])));
        assert!(!r.matches(&argv(&["cat"])));
    }

    #[test]
    fn multi_token_pattern_requires_all_tokens_to_match() {
        let r = rule(&["git", "status"]);
        assert!(r.matches(&argv(&["git", "status"])));
        assert!(r.matches(&argv(&["git", "status", "--short"])));
        assert!(!r.matches(&argv(&["git", "stash"])));
        assert!(!r.matches(&argv(&["git"])));
    }

    #[test]
    fn empty_pattern_never_matches() {
        let r = rule(&[]);
        assert!(!r.matches(&argv(&["anything"])));
        assert!(!r.matches(&argv(&[])));
    }

    #[test]
    fn empty_argv_never_matches_non_empty_pattern() {
        let r = rule(&["ls"]);
        assert!(!r.matches(&argv(&[])));
    }
}
