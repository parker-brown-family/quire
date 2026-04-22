//! Rule types produced by the Starlark policy loader.
//!
//! v0.1.0 scope: the data shape parsed out of `.policy` files. Matching
//! logic and load-time `match`/`not_match` invariant checks ship in
//! follow-up tickets; those live in `rule.rs` and `parser.rs` respectively.

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
