//! Rule types. The v0 evaluator is a stub — these structs are the target
//! shape once the Starlark loader lands in 0.1.0.

use serde::{Deserialize, Serialize};

use crate::decision::Decision;

/// A rule that matches commands whose argv starts with `pattern`.
///
/// `match` and `not_match` are invariants, not hints: on load, every entry
/// in `match` MUST classify as this rule's decision, and every entry in
/// `not_match` MUST NOT. Load-time validation fails loudly on a rule that
/// lies about what it matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrefixRule {
    pub pattern: Vec<String>,
    pub decision: Decision,
    pub justification: String,
    #[serde(default)]
    pub r#match: Vec<String>,
    #[serde(default)]
    pub not_match: Vec<String>,
}

/// Declares an executable that rules may reference by basename. The
/// evaluator resolves argv[0] to a basename and checks it against
/// registered host_executable declarations before matching prefix rules.
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
