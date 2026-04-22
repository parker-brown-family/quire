# Changelog

All notable changes to quire are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning follows
[SemVer](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Repo scaffold: Cargo manifest, CLI stub, library module layout, starter policy
  example, CI skeleton, README, MIT license, CHANGELOG.
- `quire check <cmd>` CLI surface with exit codes 0=allow, 10=prompt,
  20=forbidden, 30=no-match (stub — always returns `prompt` in 0.0.1).
- `examples/starter.policy` demonstrating `prefix_rule` and `host_executable`.

### Not yet implemented (tracked for 0.1.0)
- Starlark evaluator for policy files (`prefix_rule`, `host_executable`).
- Load-time validation of every rule's `match` and `not_match` example lists.
- Library API: `Policy::from_file`, `Policy::evaluate(&argv)`.
- Cross-platform release binaries (macOS arm64/x64, Linux x64/arm64, Windows x64).

[Unreleased]: https://github.com/parker-brown-family/quire/compare/v0.0.1...HEAD
