//! quire — Starlark-policy CLI for AI coding agents.
//!
//! This crate exposes both a library API and a `quire` binary. The v0
//! surface is deliberately small:
//!
//! - `Policy::from_file(path)` — load and validate a `.policy` file.
//! - `Policy::evaluate(argv)` — classify a command as allow / prompt /
//!   forbidden / no-match.
//! - `Decision` — the four-variant output enum.

pub mod decision;
pub mod error;
pub mod parser;
pub mod policy;
pub mod rule;

pub use decision::Decision;
pub use error::{Error, Result};
pub use policy::Policy;
pub use rule::{HostExecutable, PrefixRule, Rule};
