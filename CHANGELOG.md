# Changelog

All notable changes to quire are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning follows
[SemVer](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-22

First useful release. Policies now load, evaluate, and self-validate.

### Added
- Starlark evaluator for `.policy` files via `starlark-rust` 0.13. Two
  builtins are registered: `prefix_rule(pattern, decision, justification,
  match=[], not_match=[])` and `host_executable(name, paths)`. Declaration
  order is preserved.
- `Policy::from_file` / `Policy::from_source` / `Policy::evaluate(&argv)`.
  Matching is whole-token prefix equality — no glob, no regex, no case
  folding. First-declared matching rule wins.
- Load-time self-validation: every rule's `match=[...]` examples must
  evaluate to that rule, and every `not_match=[...]` example must not.
  A lying policy fails to load with `Error::MatchExampleDidNotMatch` or
  `Error::NotMatchExampleDidMatch` naming the rule and the offending
  example.
- `Decision::parse` + `Display`; typed `Error` enum with source-located
  variants (`InvalidDecision`, `InvalidPattern`, `InvalidExample`,
  `InvalidRule`, `Starlark`, `Io`, plus the two validation variants above).
- CLI integration tests in `tests/cli.rs` spawning the built binary
  against the real starter policy — 10 tests covering every exit code
  and the `--json` output shape.

### Changed
- `Policy::evaluate` no longer returns the 0.0.1 `Prompt` stub. It iterates
  rules in declaration order and returns the first match's decision, or
  `Decision::NoMatch` when nothing matches.
- `PrefixRule` no longer carries `match` / `not_match` fields. Those lists
  are parser-side only; the load-time validator consumes them and they
  are not part of the runtime shape.

### Dependencies
- Added `starlark = "0.13"`, `shlex = "1.3"`, `thiserror = "2"`.

## [0.0.1] - 2026-04-22

Repo scaffold: Cargo manifest, CLI stub that always returned `prompt`,
library module layout, starter policy example, CI skeleton, README, MIT
license, and this changelog.

[0.1.0]: https://github.com/parker-brown-family/quire/compare/v0.0.1...v0.1.0
[0.0.1]: https://github.com/parker-brown-family/quire/releases/tag/v0.0.1
