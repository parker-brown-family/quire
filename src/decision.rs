//! The four possible outcomes of evaluating a command against a policy.

use serde::{Deserialize, Serialize};

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
}
