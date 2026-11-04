# pr-guard

A pull request guard that runs where your code already is. It reads the diff and the PR description, asks
TS-PRGuard (a 150M-parameter model, 8-bit ONNX on the CPU) seven questions about the change, adds deterministic checks
(secrets, sensitive paths, destructive SQL, size) and lets a YAML policy decide how many reviewers the change needs,
whether it blocks, and whether it can merge on its own.

No API, no key, no telemetry: the code never leaves the machine.

## Status

Early. The CLI reads the change from git, detects the CI it runs on and writes the formats most CIs show (SARIF,
GitLab Code Quality, JUnit). Host integrations (comment, labels and status through the GitHub, GitLab and Bitbucket
APIs) and releases come next. The model is the first multi-task TS-PRGuard; the default policy runs in
shadow mode (report, never fail): set `mode: enforce` once the guard's answers on your own pull requests look right.

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

The questions and their options come from the model package when its `decision_ai.json` lists them (`tasks`);
pr-guard only asks the tasks the package lists.

### Rules before the model

What the changed files alone settle is answered by a rule, before the model and also with `--no-model`; the model is
not asked those questions.

| Answer | Rule |
|---|---|
| Type `Documentation only` | every file is documentation (`.md`, `.rst`, `.adoc`, `.txt`, `docs/` or `doc/`, `LICENSE`, `CHANGELOG`, `README`) |
| Type `Tests only` | every file is a test |
| Type `Build system, dependencies or continuous integration` | every file is a manifest, lock file, build script, Dockerfile or CI configuration |
| Type `Formatting or style only` | every hunk only moves whitespace (removed and added text equal once whitespace is dropped, and outside Python a trailing comma before `)`, `]` or `}`); formatter settings may come along |
| Tests not applicable | no production code changed beyond whitespace (docs, tests, configuration, build, formatting) |

The rules run in that order. Any other change (code, or a mix) goes to the model: whether the tests cover the changed
code, and the type among bug fix, new feature, refactoring, performance improvement and maintenance when the package
lists the type task. A package without it leaves the type empty for code changes; the type is a label only and never
weighs in the score. The text report
marks a rule answer with `(by rule)`, the JSON verdict with `change_type_source` and `tests_source` (`rule` or `model`)
next to `tests_applicable`, and the log on stderr with `pr-guard: prg-tipo rule <type>`.

## Usage

```sh
pr-guard init > .github/pr-guard.yml        # the default policy, to adjust
pr-guard check                              # HEAD against its merge base with the default branch
pr-guard check --base origin/release --head my-branch
pr-guard check --staged --no-model          # the staged changes, deterministic checks only (pre-commit)
git diff main... | pr-guard check --diff -  # any diff, from stdin or a file
pr-guard check --format sarif --output pr-guard.sarif --format markdown --output pr-guard.md
```

Exit code: 0 passed, 1 blocked by the policy, 2 usage or runtime error. Shadow mode always exits 0; `--mode shadow` or
`--mode enforce` overrides the policy's mode.

The model package (`decision_ai.json`, `tokenizer.json`, `onnx/model_8bit.onnx`, 168 MB) is read from `--model`,
`$PR_GUARD_MODEL`, or `~/.cache/pr-guard/ts-prguard` (`$PR_GUARD_CACHE` to move the cache). The cache is filled from
[typestrict/TS-PRGuard](https://huggingface.co/typestrict/TS-PRGuard) on first use, at the revision this release was
tested with, and every file is checked against its SHA-256 before it is used. `pr-guard model pull` downloads it ahead
of time (Docker images, runners without internet at check time); `$PR_GUARD_HF_ENDPOINT` points at a mirror.

### Where the change comes from

Without `--diff`, pr-guard asks the local `git`. The range ends at `--head` (default `HEAD`) and starts at the merge base
with `--base`; without `--base` it uses the branch the pull request targets when the CI says so, else `origin/HEAD`,
`main` or `master`. CI checkouts are often shallow: fetch the history (`fetch-depth: 0` with `actions/checkout`) so the
merge base exists.

The description is `--description` or `--description-file` when given; else the pull request's title and body when the
CI has them (GitHub's event payload, GitLab's merge request variables, Jenkins' `CHANGE_TITLE`); else the commit
messages of the range, newest first.

### CI detection

pr-guard reads the environment of GitHub Actions, GitLab CI, Bitbucket Pipelines, Jenkins, CircleCI, Bitrise, Buildkite
and Azure Pipelines to find the base branch, the head commit, the pull request number and the repository. It makes no
network call. With no CI it runs as on a laptop. On GitHub Actions it also prints workflow commands, so each finding
shows as an annotation on its file and line.

### Output formats

`--format` can repeat, each with its own `--output` in the same order (`-` is stdout). A single `--format` without
`--output` goes to stdout; with no `--format` the text summary goes to stdout.

| Format | What it is | Read by |
|---|---|---|
| `text` | the summary, colored on a terminal (`NO_COLOR` turns colors off) | people |
| `json` | the verdict | scripts |
| `markdown` | the sticky comment (`--comment <path>` is a shorthand) | pull request comments, job summaries |
| `sarif` | SARIF 2.1.0, one rule per check | GitHub code scanning, Azure DevOps |
| `codeclimate` | Code Climate issues | GitLab Code Quality in the merge request |
| `junit` | one test case for the verdict, each question and each finding | Jenkins, CircleCI, Bitrise, most CIs |

In JUnit a case fails at danger level and the verdict case fails when the policy blocks; nothing fails in shadow mode.

## GitHub Action

`integrations/github/action.yml` installs a released binary, caches the model in `~/.cache/pr-guard`, runs
`pr-guard check` and uploads the SARIF to code scanning. The sticky comment's markdown goes to `pr-guard.md` and to the
job summary.

```yaml
on: pull_request
permissions:
  contents: read
  security-events: write
jobs:
  pr-guard:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: typestrict-lab/pr-guard/integrations/github@main
        with:
          mode: shadow        # or enforce; empty keeps the policy's mode
          # policy: .github/pr-guard.yml
          # args: --label something
          # version: 0.1.0
```

The action never runs code from the pull request: it only reads the diff that the checkout already brought.

## pre-commit

With the `pr-guard` binary on `PATH`, add to `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/typestrict-lab/pr-guard
    rev: v0.1.0
    hooks:
      - id: pr-guard
```

The hook runs `pr-guard check --staged --no-model`: secrets, sensitive paths, destructive SQL and size on the staged
changes, without the model, in well under a second.

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
