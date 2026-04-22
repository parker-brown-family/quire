//! The four possible outcomes of evaluating a command against a policy.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Outcome of matching a command argv against a quire policy.
///
/// The variants map to distinct CLI exit codes so shell-integrating
/// harnesses can branch on them without parsing stdout:
///
///   0  Allow     — the agent may run the command unattended.
///   10 Prompt    — the harness should ask the human.
///   20 Forbidden — the harness must reject the command outright.
///   30 NoMatch   — no rule matched; the harness should fall back to its
///                  own prompt logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    Allow,
    Prompt,
    Forbidden,
    NoMatch,
}

impl Decision {
    /// CLI exit code for this decision.
    pub fn exit_code(self) -> i32 {
        match self {
            Decision::Allow => 0,
            Decision::Prompt => 10,
            Decision::Forbidden => 20,
            Decision::NoMatch => 30,
        }
    }

    /// Lowercase string form used in stdout and JSON serialization.
    pub fn as_str(self) -> &'static str {
        match self {
            Decision::Allow => "allow",
            Decision::Prompt => "prompt",
            Decision::Forbidden => "forbidden",
            Decision::NoMatch => "no-match",
        }
    }

    /// Parse a policy-source decision string. Only the three rule-level
    /// variants (`allow`, `prompt`, `forbidden`) are valid here — `no-match`
    /// is an evaluator outcome, not something a rule can declare.
    pub fn parse(raw: &str) -> Result<Self> {
        match raw {
            "allow" => Ok(Decision::Allow),
            "prompt" => Ok(Decision::Prompt),
            "forbidden" => Ok(Decision::Forbidden),
            other => Err(Error::InvalidDecision(other.to_string())),
        }
    }
}

impl fmt::Display for Decision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_are_distinct() {
        let codes = [
            Decision::Allow.exit_code(),
            Decision::Prompt.exit_code(),
            Decision::Forbidden.exit_code(),
            Decision::NoMatch.exit_code(),
        ];
        let mut sorted = codes.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), codes.len());
    }

    #[test]
    fn as_str_round_trips() {
        assert_eq!(Decision::Allow.as_str(), "allow");
        assert_eq!(Decision::Prompt.as_str(), "prompt");
        assert_eq!(Decision::Forbidden.as_str(), "forbidden");
        assert_eq!(Decision::NoMatch.as_str(), "no-match");
    }

    #[test]
    fn parse_accepts_three_rule_variants() {
        assert_eq!(Decision::parse("allow").unwrap(), Decision::Allow);
        assert_eq!(Decision::parse("prompt").unwrap(), Decision::Prompt);
        assert_eq!(Decision::parse("forbidden").unwrap(), Decision::Forbidden);
    }

    #[test]
    fn parse_rejects_no_match_and_garbage() {
        assert!(Decision::parse("no-match").is_err());
        assert!(Decision::parse("allowed").is_err());
        assert!(Decision::parse("").is_err());
    }
}
