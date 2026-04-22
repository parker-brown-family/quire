//! Starlark loader: parses a `.policy` file into rules and host-executable
//! declarations via the `prefix_rule()` and `host_executable()` builtins.
//!
//! v0.1.0 scope:
//! - `prefix_rule(pattern, decision, justification, match=[], not_match=[])`
//! - `host_executable(name, paths)`
//!
//! Each `prefix_rule` call also registers a `PendingValidation` entry so
//! that the downstream `Policy::from_file` step can run load-time
//! `match`/`not_match` invariant checks before handing the Policy to the
//! caller. The parser itself does not run those checks — it just collects.

use std::cell::RefCell;
use std::cell::RefMut;
use std::path::Path;

use starlark::any::ProvidesStaticType;
use starlark::environment::GlobalsBuilder;
use starlark::environment::Module;
use starlark::eval::Evaluator;
use starlark::starlark_module;
use starlark::syntax::AstModule;
use starlark::syntax::Dialect;
use starlark::values::list::ListRef;
use starlark::values::list::UnpackList;
use starlark::values::none::NoneType;
use starlark::values::Value;

use crate::decision::Decision;
use crate::error::{Error, Result};
use crate::rule::{HostExecutable, PrefixRule};

/// One `prefix_rule` call's example lists, paired with the rule they came
/// from so the validator can report exact source context on failure.
#[derive(Debug, Clone)]
pub struct PendingValidation {
    pub rule_index: usize,
    pub matches: Vec<Vec<String>>,
    pub not_matches: Vec<Vec<String>>,
}

/// Everything the Starlark loader produces. Consumed by `Policy::from_file`.
#[derive(Debug, Default)]
pub struct ParsedPolicy {
    pub rules: Vec<PrefixRule>,
    pub host_executables: Vec<HostExecutable>,
    pub pending_validations: Vec<PendingValidation>,
}

#[derive(Debug, Default, ProvidesStaticType)]
struct Builder {
    rules: Vec<PrefixRule>,
    host_executables: Vec<HostExecutable>,
    pending_validations: Vec<PendingValidation>,
}

impl Builder {
    fn add_rule(&mut self, rule: PrefixRule) -> usize {
        let idx = self.rules.len();
        self.rules.push(rule);
        idx
    }
}

/// Parse a policy file from disk. Returns a fully loaded `ParsedPolicy`
/// with rules, host executables, and pending example validations, but does
/// NOT run the validations (the caller in `Policy::from_file` does).
pub fn parse_file(path: impl AsRef<Path>) -> Result<ParsedPolicy> {
    let path = path.as_ref();
    let source = std::fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.display().to_string(),
        source,
    })?;
    parse_source(&path.display().to_string(), &source)
}

/// Parse a policy from an in-memory source string. Useful for tests and
/// embedded callers that already have the Starlark source in hand.
pub fn parse_source(identifier: &str, source: &str) -> Result<ParsedPolicy> {
    let dialect = Dialect::Standard.clone();
    let ast =
        AstModule::parse(identifier, source.to_string(), &dialect).map_err(Error::Starlark)?;

    let globals = GlobalsBuilder::standard().with(policy_builtins).build();
    let module = Module::new();
    let builder = RefCell::new(Builder::default());
    {
        let mut eval = Evaluator::new(&module);
        eval.extra = Some(&builder);
        eval.eval_module(ast, &globals).map_err(Error::Starlark)?;
    }
    let Builder {
        rules,
        host_executables,
        pending_validations,
    } = builder.into_inner();
    Ok(ParsedPolicy {
        rules,
        host_executables,
        pending_validations,
    })
}

fn builder_mut<'v, 'a>(eval: &Evaluator<'v, 'a, '_>) -> RefMut<'a, Builder> {
    eval.extra
        .as_ref()
        .expect("Evaluator.extra must be set to the policy Builder")
        .downcast_ref::<RefCell<Builder>>()
        .expect("Evaluator.extra must contain a RefCell<Builder>")
        .borrow_mut()
}

fn parse_pattern<'v>(pattern: UnpackList<Value<'v>>) -> Result<Vec<String>> {
    let tokens: Vec<String> = pattern
        .items
        .into_iter()
        .map(|v| {
            v.unpack_str().map(str::to_string).ok_or_else(|| {
                Error::InvalidPattern(format!(
                    "pattern element must be a string (got {})",
                    v.get_type()
                ))
            })
        })
        .collect::<Result<_>>()?;
    if tokens.is_empty() {
        return Err(Error::InvalidPattern("pattern cannot be empty".into()));
    }
    Ok(tokens)
}

fn parse_examples<'v>(examples: UnpackList<Value<'v>>) -> Result<Vec<Vec<String>>> {
    examples.items.into_iter().map(parse_example).collect()
}

fn parse_example<'v>(value: Value<'v>) -> Result<Vec<String>> {
    if let Some(raw) = value.unpack_str() {
        let tokens = shlex::split(raw).ok_or_else(|| Error::InvalidExample {
            raw: raw.to_string(),
            reason: "invalid shell syntax".into(),
        })?;
        if tokens.is_empty() {
            return Err(Error::InvalidExample {
                raw: raw.to_string(),
                reason: "empty".into(),
            });
        }
        Ok(tokens)
    } else if let Some(list) = ListRef::from_value(value) {
        let tokens: Vec<String> = list
            .content()
            .iter()
            .map(|v| {
                v.unpack_str()
                    .map(str::to_string)
                    .ok_or_else(|| Error::InvalidExample {
                        raw: format!("{value:?}"),
                        reason: format!("list elements must be strings (got {})", v.get_type()),
                    })
            })
            .collect::<Result<_>>()?;
        if tokens.is_empty() {
            return Err(Error::InvalidExample {
                raw: format!("{value:?}"),
                reason: "empty list".into(),
            });
        }
        Ok(tokens)
    } else {
        Err(Error::InvalidExample {
            raw: format!("{value:?}"),
            reason: format!(
                "example must be a string or list of strings (got {})",
                value.get_type()
            ),
        })
    }
}

#[starlark_module]
fn policy_builtins(builder: &mut GlobalsBuilder) {
    fn prefix_rule<'v>(
        pattern: UnpackList<Value<'v>>,
        decision: &'v str,
        justification: &'v str,
        #[starlark(require = named)] r#match: Option<UnpackList<Value<'v>>>,
        #[starlark(require = named)] not_match: Option<UnpackList<Value<'v>>>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> anyhow::Result<NoneType> {
        let pattern_tokens = parse_pattern(pattern)?;
        let decision = Decision::parse(decision)?;
        let justification = justification.trim();
        if justification.is_empty() {
            return Err(Error::InvalidRule("justification cannot be empty".into()).into());
        }
        let matches: Vec<Vec<String>> =
            r#match.map(parse_examples).transpose()?.unwrap_or_default();
        let not_matches: Vec<Vec<String>> = not_match
            .map(parse_examples)
            .transpose()?
            .unwrap_or_default();

        let rule = PrefixRule {
            pattern: pattern_tokens,
            decision,
            justification: justification.to_string(),
        };
        let mut b = builder_mut(eval);
        let idx = b.add_rule(rule);
        b.pending_validations.push(PendingValidation {
            rule_index: idx,
            matches,
            not_matches,
        });
        Ok(NoneType)
    }

    fn host_executable<'v>(
        name: &'v str,
        paths: UnpackList<Value<'v>>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> anyhow::Result<NoneType> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::InvalidRule("host_executable name cannot be empty".into()).into());
        }
        let paths_vec: Vec<String> = paths
            .items
            .into_iter()
            .map(|v| {
                v.unpack_str().map(str::to_string).ok_or_else(|| {
                    Error::InvalidRule(format!(
                        "host_executable paths must be strings (got {})",
                        v.get_type()
                    ))
                })
            })
            .collect::<std::result::Result<_, Error>>()?;
        builder_mut(eval).host_executables.push(HostExecutable {
            name: name.to_string(),
            paths: paths_vec,
        });
        Ok(NoneType)
    }
}

/// Run the `match` / `not_match` self-validation pass for a freshly parsed
/// policy. This is what makes quire's policy files contain their own
/// regression test: a rule that lies about what it matches does not load.
///
/// Semantics:
/// - Every `match` example must evaluate to the rule that declared it. If
///   Policy::evaluate on the example picks a different rule (or no rule),
///   the load fails with `Error::MatchExampleDidNotMatch`.
/// - Every `not_match` example must NOT evaluate to the rule that declared
///   it. It may match a different rule or no rule at all. Otherwise the
///   load fails with `Error::NotMatchExampleDidMatch`.
pub fn validate_examples(rules: &[PrefixRule], pending: &[PendingValidation]) -> Result<()> {
    for pv in pending {
        let owner = &rules[pv.rule_index];
        let owner_id = format_rule_id(owner);

        for example in &pv.matches {
            let actual = first_matching_index(rules, example);
            match actual {
                Some(idx) if idx == pv.rule_index => {}
                Some(idx) => {
                    return Err(Error::MatchExampleDidNotMatch {
                        rule: owner_id,
                        example: example.join(" "),
                        actual: format!("rule `{}` matched first", format_rule_id(&rules[idx])),
                    });
                }
                None => {
                    return Err(Error::MatchExampleDidNotMatch {
                        rule: owner_id,
                        example: example.join(" "),
                        actual: "no-match".into(),
                    });
                }
            }
        }

        for example in &pv.not_matches {
            let actual = first_matching_index(rules, example);
            if actual == Some(pv.rule_index) {
                return Err(Error::NotMatchExampleDidMatch {
                    rule: owner_id,
                    example: example.join(" "),
                });
            }
        }
    }
    Ok(())
}

fn first_matching_index(rules: &[PrefixRule], argv: &[String]) -> Option<usize> {
    rules.iter().position(|r| r.matches(argv))
}

fn format_rule_id(rule: &PrefixRule) -> String {
    rule.pattern.join(" ")
}
