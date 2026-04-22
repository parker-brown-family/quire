//! Typed errors raised by policy loading and evaluation.

use starlark::Error as StarlarkError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid decision `{0}` (expected one of: allow, prompt, forbidden)")]
    InvalidDecision(String),

    #[error("invalid pattern: {0}")]
    InvalidPattern(String),

    #[error("invalid example `{raw}`: {reason}")]
    InvalidExample { raw: String, reason: String },

    #[error("invalid rule: {0}")]
    InvalidRule(String),

    #[error(
        "rule `{rule}` claims `match={example:?}` but evaluating that command did not match it (actual: {actual})"
    )]
    MatchExampleDidNotMatch {
        rule: String,
        example: String,
        actual: String,
    },

    #[error(
        "rule `{rule}` claims `not_match={example:?}` but evaluating that command DID match it"
    )]
    NotMatchExampleDidMatch { rule: String, example: String },

    #[error("starlark error: {0}")]
    Starlark(StarlarkError),

    #[error("i/o error reading `{path}`: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}
