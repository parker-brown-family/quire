//! Policy loading and evaluation.
//!
//! v0 is a stub: `Policy::from_file` returns an empty policy regardless of
//! input, and `Policy::evaluate` always returns `Decision::Prompt`. The real
//! Starlark loader lands in 0.1.0.

use std::path::Path;

use anyhow::Result;

use crate::decision::Decision;
use crate::rule::{HostExecutable, Rule};

#[derive(Debug, Default, Clone)]
pub struct Policy {
    pub rules: Vec<Rule>,
    pub host_executables: Vec<HostExecutable>,
    pub source_path: Option<String>,
}

impl Policy {
    /// Load a policy from a `.policy` file.
    ///
    /// 0.0.1: returns an empty Policy with `source_path` set.
    /// 0.1.0: will parse the file as Starlark, evaluate `prefix_rule` and
    /// `host_executable` calls, and validate every rule's `match` /
    /// `not_match` invariants.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        Ok(Policy {
            rules: Vec::new(),
            host_executables: Vec::new(),
            source_path: Some(path.display().to_string()),
        })
    }

    /// Classify a command against this policy.
    ///
    /// 0.0.1: always returns `Decision::Prompt` (safe default — asks the
    /// human about everything).
    /// 0.1.0: will return the first matching rule's decision, or
    /// `Decision::NoMatch` if nothing matches.
    pub fn evaluate(&self, _argv: &[String]) -> Decision {
        Decision::Prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_policy_loads_with_source_path() {
        let p = Policy::from_file("examples/starter.policy").unwrap();
        assert!(p.source_path.is_some());
        assert!(p.rules.is_empty());
    }

    #[test]
    fn stub_evaluate_always_prompts() {
        let p = Policy::default();
        let argv = vec!["ls".to_string(), "-la".to_string()];
        assert_eq!(p.evaluate(&argv), Decision::Prompt);
    }
}
