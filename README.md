# pr-guard

A pull request guard that runs where your code already is. It reads the diff and the PR description, asks
TS-PRGuard (a 150M-parameter model, 8-bit ONNX on the CPU) seven questions about the change, adds deterministic checks
(secrets, sensitive paths, destructive SQL, size) and lets a YAML policy decide how many reviewers the change needs,
whether it blocks, and whether it can merge on its own.

No API, no key, no telemetry: the code never leaves the machine.

## Status

Early. The CLI works locally on a diff; reading git, CI detection, host integrations (GitHub, GitLab, Bitbucket) and
releases come next. The model is the first multi-task TS-PRGuard; its answers are not yet tuned into blocking
thresholds, so start every repository in shadow mode.

## The questions

| Question | Used as |
|---|---|
| Does the change do what the description says? | weighs in the score |
| Does it introduce a security vulnerability? | warning |
| Does it break compatibility for users? | warning |
| Do the tests cover the changed code? | warning |
| How hard is it to review? (5 bands) | weighs in the score |
| What kind of change is it? | label only |
| Does it break the public API? | reported, not used yet |

## Usage

```sh
pr-guard init > .github/pr-guard.yml        # the default policy, to adjust
git diff main... > pr.diff
pr-guard check --diff pr.diff --description-file pr-body.md --comment comment.md
pr-guard check --diff pr.diff --no-model    # deterministic checks only (pre-commit)
```

Exit code: 0 passed, 1 blocked by the policy, 2 usage or runtime error. Shadow mode always exits 0.

The model package (`decision_ai.json`, `tokenizer.json`, `onnx/model_8bit.onnx`) is read from `--model`,
`$PR_GUARD_MODEL`, or `~/.cache/pr-guard/ts-prguard` (`$PR_GUARD_CACHE` to move the cache).

## Examples

`examples/` has small projects in several languages with pull requests that plant real problems (SQL injection, a
removed permission check, a renamed public function, a description that says something else) next to clean ones.

```sh
bash examples/run.sh
```

## Development

```sh
cargo test                                         # parity tests skip without the weights
PR_GUARD_MODEL=/path/to/model cargo test --release
```

## License

Apache-2.0
