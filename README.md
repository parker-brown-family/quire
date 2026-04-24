# quire

**Starlark-policy CLI for AI coding agents — stop approving `ls` forever.**

```sh
$ cat policy.quire
prefix_rule(pattern=["ls"], decision="allow",
            justification="ls is read-only.",
            match=["ls", "ls -la"], not_match=["lsof"])

$ quire check --policy policy.quire -- ls -la
allow
$ echo $?
0
```

One policy file. One CLI. Four exit codes. Wire it into your agent harness
once and your AI stops asking permission for every `grep`.

---

## Table of Contents

- [Solution](#solution)
- [The problem it solves](#the-problem-it-solves)
- [How it works](#how-it-works)
- [Adopt it in your existing harness](#adopt-it-in-your-existing-harness)
- [Adopt it if you are new to AI-assisted coding](#adopt-it-if-you-are-new-to-ai-assisted-coding)
- [Reference](#reference)
- [Prior art & credits](#prior-art--credits)
- [License](#license)

---

## Solution

**One file describes what your agent may do. Every command gets a yes / ask /
no / shrug answer in single-digit milliseconds.**

The policy is a short Starlark file. Each rule says three things:

1. what command prefix it matches (`pattern`),
2. what the agent may do with it (`decision`: `allow`, `prompt`, or
   `forbidden`),
3. why — in prose the agent can show the human when it asks.

Every rule must also ship a list of examples it **does** match and a list it
**does not** match. On load, quire checks them. A rule that lies about what
it matches does not load.

```python
# policy.quire
prefix_rule(
    pattern = ["git", "push"],
    decision = "prompt",
    justification = "git push mutates the remote — always ask the human.",
    match = ["git push", "git push origin main"],
    not_match = ["git push-option", "git status"],
)
```

The CLI is one verb:

```sh
quire check --policy policy.quire -- <cmd> [args...]
```

Exit code tells your harness what to do, no stdout parsing needed.

| Exit | Meaning     | What your harness should do                          |
| ---: | ----------- | ---------------------------------------------------- |
|    0 | `allow`     | Run the command. Do not prompt the human.            |
|   10 | `prompt`    | Ask the human first.                                 |
|   20 | `forbidden` | Reject the command outright. Do not ask.             |
|   30 | `no-match`  | No rule matched. Fall back to the harness default.   |

That is the whole surface.

---

## The problem it solves

If you have used Claude Code, Cursor's agent mode, Aider, Codex, or any
other shell-tool-wielding AI, you know this loop:

- The agent wants to run `ls`.
- The tool asks you: **"Allow this command?"**
- You say yes.
- Ten seconds later the agent wants to run `ls src/`. Different argv,
  same question.
- You say yes.
- Half an hour later you have clicked "Allow" sixty times for read-only
  commands that could never have hurt anything, and you still get jumpscared
  by every `rm` and every `git push`.

So you reach for the allow-list file your harness ships with. Fifteen
minutes later you have a 400-line cryptic YAML of command fragments, no
test for whether any of them do what you meant, a handful of rules that
accidentally allow `rm -rf` because `rm` was in the prefix list, and zero
way to share it with a teammate because half of it refers to your dotfiles.

**quire is the allow-list file, taken seriously.** One format. One tool.
Rules that validate themselves. A clear contract with your harness. And a
standard enough starter policy that your first ten minutes of "let me
approve every read command" do not have to happen.

---

## How it works

### 1. Policies are Starlark, not YAML

Starlark is a deterministic Python-like config language originally built for
Bazel. It has functions, lists, strings, and booleans — no I/O, no
recursion, no surprises. You get:

- Variables and helpers (define a list of safe read-only binaries once,
  reference it ten times).
- A real parser with real error messages.
- File composition (`include("./shared.quire")`, planned for 0.2).

### 2. Rules match by argv prefix

`prefix_rule(pattern=["git", "status"], ...)` matches any argv whose first
tokens are `git status`. It does **not** match `git stash`, `git status-x`,
or `mygit status`. Prefix matching is unambiguous — no regex edge cases, no
shell-quoting gotchas.

### 3. Every rule self-validates on load

```python
prefix_rule(
    pattern   = ["ls"],
    decision  = "allow",
    match     = ["ls", "ls -la"],   # MUST match this rule
    not_match = ["lsof", "lsblk"],  # MUST NOT match this rule
    ...
)
```

When quire loads the policy, it runs every `match` entry through the
evaluator. If any of them fails to match this rule — or worse, matches a
*different* rule — the load fails loudly with a line number. Same for
`not_match`. This is the killer feature: **your policy file contains its
own regression test.**

### 4. Four outcomes, four exit codes

Every command gets exactly one of `allow`, `prompt`, `forbidden`, or
`no-match`. No "maybe if the file exists," no "allow but log." The harness
knows exactly what to do before the command runs.

---

## Adopt it in your existing harness

quire is built for harness authors first. It assumes you already have a
permission layer and a tool loop. You need it to consult an external
authority cheaply, get a single answer, and move on.

### The universal integration

```sh
if quire check --policy "$QUIRE_POLICY" -- "$@"; then
    exec "$@"                 # decision: allow (exit 0)
else
    code=$?
    case $code in
        10) ask_human "$@" ;; # prompt
        20) reject "$@" ;;    # forbidden
        30) harness_default_prompt "$@" ;;  # no-match — fall through
        *)  echo "quire usage error" >&2; exit "$code" ;;
    esac
fi
```

That's it. Three-line wrapper, works with every harness that can shell
out.

### Claude Code

Claude Code reads a `PreToolUse` hook. Point it at quire:

```sh
# ~/.claude/hooks/pre-tool-use
#!/usr/bin/env bash
cmd_json="$1"
argv=$(echo "$cmd_json" | jq -r '.command | @sh')
eval "set -- $argv"
quire check --policy ~/.config/quire/policy -- "$@"
```

quire's exit code becomes the hook's exit code — Claude Code respects it.

### Cursor / Aider / OpenCode

Any harness that exposes a shell hook or pre-execution callback works the
same way. Configure the hook to invoke `quire check --policy <file> -- "$@"`
and branch on `$?`.

### Library API (Rust)

Embedding directly? Skip the process and link the library:

```rust
use quire::{Policy, Decision};

let policy = Policy::from_file("policy.quire")?;
let argv = std::env::args().skip(1).collect::<Vec<_>>();

match policy.evaluate(&argv) {
    Decision::Allow     => run(&argv),
    Decision::Prompt    => ask_human(&argv),
    Decision::Forbidden => reject(&argv),
    Decision::NoMatch   => fall_through(&argv),
}
```

Policies hot-reload safely — each `Policy` is an immutable snapshot, so
re-load from disk whenever you like.

### Node wrapper (planned)

`@quire/node` will ship in 0.2 as a thin spawn-based wrapper around the
binary. Until then, shell out:

```ts
import { execFileSync } from "node:child_process";

export function check(argv: string[], policy: string): Decision {
    const res = execFileSync("quire", ["check", "--policy", policy, "--", ...argv], {
        stdio: ["ignore", "pipe", "inherit"],
    });
    const code = 0; // execFileSync throws on non-zero — wrap appropriately
    // map exit code to Decision in your wrapper
}
```

### What quire does NOT do

- **Does not run the command.** quire is stateless advice; your harness
  stays in charge.
- **Does not sandbox.** No seccomp, no apparmor, no containerization. Pair
  quire with your OS's sandbox if you need isolation.
- **Does not phone home.** No telemetry, no network, no config server.
  Policies are local files.

---

## Adopt it if you are new to AI-assisted coding

You probably have not built a harness. You are using Claude Code, Cursor,
or a similar tool out of the box, and it keeps stopping to ask you about
every little command. Here is the ten-minute path to make it stop.

### 1. Install quire

Homebrew, crates.io, or grab a release binary from
[Releases](https://github.com/parker-brown-family/quire/releases).

```sh
brew install parker-brown-family/tap/quire
# or
cargo install quire
```

### 2. Copy the starter policy

```sh
mkdir -p ~/.config/quire
cp examples/starter.policy ~/.config/quire/policy
```

You now have sane defaults for `ls`, `cat`, `git status`, `git push`, and
`rm -rf`. Read the file — it is thirty lines and every rule tells you
exactly what it does and why.

### 3. Wire it into your harness

Pick your harness from the [Existing harness](#adopt-it-in-your-existing-harness)
section above and drop in the three-line wrapper or hook script.

If your harness does not have a pre-exec hook, quire cannot help you
directly — but open an issue with your harness asking for one. "My tool
respects quire" is about to become a checkbox people care about.

### 4. Tune the policy as you go

The most common edits:

- **Add a read-only command you use constantly.** Copy the `ls` rule,
  change the pattern and the `match` entries, save.
- **Forbid something that scared you.** Copy the `rm -rf` rule, change
  the pattern, keep `decision = "forbidden"`.
- **Promote a prompt to allow when you trust it.** Change
  `decision = "prompt"` to `decision = "allow"`. Add `match` entries for
  the exact shapes you trust.

After every edit, validate the policy:

```sh
quire check --policy ~/.config/quire/policy -- ls   # sanity
```

If the policy has a broken `match` or `not_match` example, the load
fails and you see the line number.

### 5. What if no rule matches?

quire exits 30 (`no-match`). Your harness falls back to its own
permission prompt — the one you already know. quire does not make you
worse off. It only adds a fast-path for the commands you have opinions
about.

---

## Reference

### Policy file grammar

A policy file is a Starlark module. v0 defines two top-level functions:

```python
prefix_rule(
    pattern:       list[str],         # argv prefix; required
    decision:      str,               # "allow" | "prompt" | "forbidden"; required
    justification: str,               # human-readable reason; required
    match:         list[str] = [],    # commands that MUST match this rule
    not_match:     list[str] = [],    # commands that MUST NOT match this rule
)

host_executable(
    name:  str,             # basename — e.g. "git", "node"
    paths: list[str],       # absolute paths where this binary is allowed to live
)
```

Strings in `match` / `not_match` are parsed with POSIX shell splitting
(`shlex`), so `"ls -la /tmp"` becomes `["ls", "-la", "/tmp"]` before
evaluation.

### Matching algorithm

For a command with argv `[a0, a1, ..., aN]`:

1. Resolve `a0` to its basename and look up the matching `host_executable`.
   Failing that, use `a0` as-is. Note that `host_executable` declarations are parsed and validated but not yet consulted during evaluation — path-based resolution is planned for 0.2.
2. For each `prefix_rule` in file order, check whether `pattern[i] == argv[i]`
   for all `i` in `0..len(pattern)`. First match wins.
3. If no rule matches, return `no-match`.

### Exit codes

| Code | Name        |
| ---: | ----------- |
|    0 | `allow`     |
|   10 | `prompt`    |
|   20 | `forbidden` |
|   30 | `no-match`  |
|    2 | usage error or policy load failure (stderr explains) |

### CLI

```
quire check --policy <FILE> [--json] -- <CMD> [ARGS...]

    --policy <FILE>    path to a .policy file
    --json             emit JSON to stdout instead of a decision word
    --                 everything after this is the command argv
```

### Library API (Rust)

See [docs.rs/quire](https://docs.rs/quire). Summary:

```rust
pub enum Decision { Allow, Prompt, Forbidden, NoMatch }

impl Decision {
    pub fn exit_code(self) -> i32;
    pub fn as_str(self) -> &'static str;
}

pub struct Policy { /* ... */ }

impl Policy {
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self>;
    pub fn evaluate(&self, argv: &[String]) -> Decision;
}
```

### Architecture

- `src/decision.rs`  — `Decision` enum + exit-code / as_str impls.
- `src/rule.rs`      — `PrefixRule`, `HostExecutable`, `Rule` structs.
- `src/policy.rs`    — `Policy::from_file`, `Policy::evaluate`.
- `src/main.rs`      — clap CLI; thin wrapper over the library.

### Contributing

Issues and PRs welcome. Please run `cargo fmt`, `cargo clippy
--all-targets -- -D warnings`, and `cargo test` before pushing.

Design decisions live in [`docs/design.md`](./docs/design.md) (written as
they are made — 0.1.0 will land the first entries).

---

## Prior art & credits

quire exists because **OpenAI's Codex team already solved this problem
once** in their `codex-rs/execpolicy` crate, and then buried it inside a
larger harness. Their prefix-rule schema, the `match` / `not_match`
self-validation idea, and the decision enum are directly inspired by
their work. If you want a more capable, less separable version, read
[their source](https://github.com/openai/codex/tree/main/codex-rs/execpolicy).

quire's contribution is **separation**. The single most-reusable piece of
harness engineering should not require you to adopt a harness.

Related work worth knowing:

- [Claude Code](https://docs.anthropic.com/claude/docs/claude-code) — hooks
  API this integrates with.
- [Aider](https://aider.chat/) — explicit confirm workflow; complements quire.
- [Starlark](https://github.com/bazelbuild/starlark) — the language; quire
  uses [starlark-rust](https://github.com/facebook/starlark-rust) at load time.

---

## License

MIT. See [LICENSE](./LICENSE).

Copyright (c) 2026 Parker Brown Family Sports.
