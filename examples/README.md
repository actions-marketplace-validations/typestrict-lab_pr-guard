# pr-guard examples

Six small but realistic projects, one per language, each with a handful of
scenario pull requests. Some scenarios are clean, others plant the kind of
problem a careful human reviewer would catch: a security hole, a breaking
change, a description that does not match the diff, a logic bug with no tests.
Run pr-guard over all of them to see how it behaves on concrete cases.

| Project | Language | What it is |
|---|---|---|
| `python-todo-api` | Python (Flask, SQLite) | Per-user to-do list API with bearer tokens |
| `typescript-cart` | TypeScript (vitest) | Shopping cart domain logic in integer cents, coupons, tax client |
| `go-ratelimit` | Go | Per-key token bucket limiter and `net/http` middleware |
| `rust-config` | Rust (serde, toml) | Loads per-profile TOML settings with validation |
| `kotlin-wallet` | Kotlin (JVM) | Wallets with PIN-protected transfers and currency conversion |
| `ruby-blog` | Ruby (Sinatra) | JSON blog API with an admin token |

## Layout

```
examples/
  run.sh                 runs pr-guard on every scenario
  check-apply.sh         checks that every change.diff applies to its project
  <project>/             the project at its base state, with its tests
    scenarios/<NN-slug>/
      change.diff        the pull request, in `git diff` format
      description.md     PR title on the first line, then the body
      expected.yaml      what a good reviewer should conclude
      comment.md         written by run.sh (git ignored)
```

`expected.yaml` uses the same questions pr-guard answers:

- `description_matches`: `yes` or `no`
- `vulnerability`: `yes` or `no`, with `cwe` when `yes`
- `breaking`: `yes` or `no`
- `tests`: `covers`, `missing` or `not_applicable`
- `type`: one of Bug fix, New feature, Refactoring without behavior change,
  Performance improvement, Tests only, Build system, dependencies or continuous
  integration, Maintenance or tooling, Formatting or style only, Documentation only
- `complexity`: 0 (trivial) to 4 (very large)
- `note`: one line on what is planted, or why the change is clean

## Running

From the repository root:

```sh
examples/run.sh                                  # builds with cargo, runs every scenario
PR_GUARD=target/release/pr-guard examples/run.sh # use an already built binary
examples/run.sh kotlin-wallet                    # one project only
examples/check-apply.sh                          # every diff applies with git apply --check
```

For each scenario `run.sh` prints the PR title, the expected note, the
`pr-guard:` lines (one per question, with the model's confidence) and whether
the run passed or was blocked. A blocked run (exit 1) is a valid outcome and
the loop keeps going. The sticky comment goes to `<scenario>/comment.md`.

`run.sh` points `--policy` at a path that does not exist, so pr-guard uses its
default policy. Set `PR_GUARD_POLICY` to try your own policy file.

To look at a scenario applied to its project:

```sh
cp -r examples/go-ratelimit /tmp/rl && cd /tmp/rl && git init -q
git apply scenarios/02-trust-forwarded-for/change.diff && go test ./...
```

## Coverage

Across the set, each kind of scenario appears in at least three languages:

| Kind | Scenarios |
|---|---|
| Clean change with tests | every project's `01-*`, plus the refactors |
| Security bug | SQL injection (python 02), missing ownership check (python 03), hardcoded secret (typescript 02), spoofable client IP (go 02), fail open (go 06), path traversal (rust 02), disabled TLS verification (kotlin 02), weak PIN hashing (kotlin 03), unsafe deserialization (ruby 02), missing authentication (ruby 03) |
| Breaking change | renamed JSON field (python 06, kotlin 05), renamed method with new signature (typescript 03), changed constructor signature (go 03), renamed config key and new default (rust 03), stored PIN hashes no longer verify (kotlin 03) |
| Description mismatch | python 03, typescript 04, go 06, ruby 03 |
| Logic bug without tests | python 04, typescript 06, go 04, rust 05, kotlin 04, ruby 04 |
| Docs or formatting only | python 05, typescript 05, go 05, rust 04, kotlin 06, ruby 05 |
| Larger refactor across files | python 07, typescript 07, rust 06, ruby 06 |

## Scenarios

### go-ratelimit

| Scenario | Expected verdict | Planted issue |
|---|---|---|
| `01-retry-after-header` | should pass (New feature, complexity 1) | Clean change: additive Take method, Retry-After header, new test. |
| `02-trust-forwarded-for` | CWE-348 (Bug fix, complexity 1) | X-Forwarded-For is trusted from any client with no trusted proxy list, so anyone can send a fresh value per request and bypass the limit. |
| `03-config-struct` | breaking (Refactoring without behavior change, complexity 1) | Exported constructor New changes signature from (float64, int) to (Config): every importer breaks at compile time. |
| `04-allow-n` | tests missing (New feature, complexity 1) | Off-by-one: AllowN checks tokens < n-1 instead of tokens < n, so it allows a request one token short and drives the bucket negative; no tests. |
| `05-readme-tuning` | should pass (Documentation only, complexity 0) | Docs only: README gains tuning and proxy guidance. |
| `06-better-log-message` | description mismatch, CWE-636, tests missing (Bug fix, complexity 0) | Besides the log text, a request whose key cannot be derived is now passed to the handler instead of rejected (fail open), which the description does not mention. |

### kotlin-wallet

| Scenario | Expected verdict | Planted issue |
|---|---|---|
| `01-withdraw` | should pass (New feature, complexity 1) | Clean change: withdraw reuses the same owner and PIN checks, validates the amount, and has tests. |
| `02-staging-self-signed-cert` | CWE-295, tests missing (Bug fix, complexity 1) | Installs a trust-all X509TrustManager as the default for every environment, disabling certificate validation for production rate calls too. |
| `03-faster-pin-check` | CWE-916, breaking, tests missing (Performance improvement, complexity 1) | Replaces PBKDF2 with a single salted MD5, so a leaked table of 4 to 8 digit PINs is cracked instantly; existing stored hashes also stop verifying. |
| `04-simplify-convert` | tests missing (Refactoring without behavior change, complexity 0) | Money converted with Double and truncated by toLong() instead of BigDecimal with HALF_EVEN, so credited amounts can lose a cent; no test for a rounding case. |
| `05-balance-cents-json` | breaking (Refactoring without behavior change, complexity 0) | Public JSON field balance renamed to balance_cents with no alias or versioning: every API client reading balance breaks. |
| `06-kdoc` | should pass (Documentation only, complexity 0) | Docs only: KDoc comments and README text. |

### python-todo-api

| Scenario | Expected verdict | Planted issue |
|---|---|---|
| `01-filter-by-done` | should pass (New feature, complexity 1) | Clean change: parameterized filter, input validated, tests and README updated. |
| `02-search-by-title` | CWE-89 (New feature, complexity 1) | The search term is interpolated into the SQL with an f-string instead of a bound parameter (SQL injection). |
| `03-readme-typo` | description mismatch, CWE-639, tests missing (Bug fix, complexity 1) | Says README only, but also drops the _owned() check in delete_todo, so any user can delete anyone's todo by id. |
| `04-per-page-param` | tests missing (New feature, complexity 1) | The offset still uses PAGE_SIZE instead of per_page, so with per_page=50 page 2 repeats items 21 to 50; per_page <= 0 is not rejected (LIMIT -1 means no limit in SQLite); no tests. |
| `05-docs-auth` | should pass (Documentation only, complexity 0) | Docs only: README gains an authentication section and curl examples. |
| `06-rename-done-to-completed` | breaking (Refactoring without behavior change, complexity 1) | Public JSON field done renamed to completed in responses and PATCH input with no fallback: existing clients break. |
| `07-repository-layer` | should pass (Refactoring without behavior change, complexity 2) | Clean multi-file refactor: SQL moves to a repository, ownership checks are kept (and tightened), new unit test. |

### ruby-blog

| Scenario | Expected verdict | Planted issue |
|---|---|---|
| `01-filter-by-author` | should pass (New feature, complexity 0) | Clean change: optional filter on already published posts, with a test. |
| `02-reader-preferences` | CWE-502 (New feature, complexity 1) | The prefs cookie is decoded with Marshal.load, so any visitor can send a crafted cookie and get remote code execution. |
| `03-bump-sinatra` | description mismatch, CWE-862, tests missing (Build system, dependencies or continuous integration, complexity 1) | Says dependency bump, but also replaces the per-route require_admin! calls with before("/admin/posts"), which does not match /admin/posts/:id, so DELETE is left unauthenticated. |
| `04-paginate-posts` | tests missing (New feature, complexity 0) | X-Total-Pages uses integer division, so a partial last page is never linked (25 posts report 2 pages, fewer than 10 report 0); no tests for pagination. |
| `05-rubocop-layout` | should pass (Formatting or style only, complexity 0) | Formatting only: long lines wrapped, arguments realigned, linter config added. |
| `06-post-serializer` | should pass (Refactoring without behavior change, complexity 2) | Clean refactor across four files plus a new test; same keys in API responses and on disk. |

### rust-config

| Scenario | Expected verdict | Planted issue |
|---|---|---|
| `01-validate-settings` | should pass (Bug fix, complexity 1) | Clean change: validation after parsing, new variant on a non_exhaustive enum, tests for both rules. |
| `02-nested-profiles` | CWE-22 (New feature, complexity 1) | Profile validation is removed entirely, so ../ (or an absolute path) escapes the config dir; the traversal test is deleted instead of adapted. |
| `03-rename-listen-addr` | breaking (Refactoring without behavior change, complexity 1) | Config key listen_addr renamed to bind with deny_unknown_fields and no alias, and the default port changes: existing config files fail to load. |
| `04-cargo-fmt` | should pass (Formatting or style only, complexity 0) | Formatting only: rustfmt rewraps long lines in loader.rs, duration.rs and the tests, same tokens. |
| `05-hours-unit` | tests missing (New feature, complexity 0) | The new h unit multiplies by 60 instead of 3600, so 6h means six minutes; the existing unit test is not extended. |
| `06-profile-name-type` | should pass (Refactoring without behavior change, complexity 2) | Clean refactor across five files: same validation rule moved into a newtype, adapter moved, new unit test. |

### typescript-cart

| Scenario | Expected verdict | Planted issue |
|---|---|---|
| `01-max-quantity-per-sku` | should pass (New feature, complexity 1) | Clean change: new cap enforced on the merged quantity, with a test for the boundary. |
| `02-tax-client-fallback-key` | CWE-798, tests missing (Bug fix, complexity 0) | A live API key is hardcoded as the fallback for TAX_API_KEY and committed to the repo. |
| `03-rename-additem-to-add` | breaking (Refactoring without behavior change, complexity 1) | Public method addItem removed and replaced by add with a different signature; acknowledged with a major bump, consumers break. |
| `04-extract-coupon-helper` | description mismatch, tests missing (Refactoring without behavior change, complexity 1) | Claims no behavior change but the minSubtotal check is dropped, so coupons apply to any order; the test asserting it is quietly removed. |
| `05-prettier` | should pass (Formatting or style only, complexity 1) | Formatting only: prettier config plus rewrapped lines, no logic change. |
| `06-amounts-with-separators` | tests missing (New feature, complexity 1) | Money parsed with parseFloat and Math.floor(value * 100): 0.29 becomes 28 cents; negatives and trailing garbage like 12abc are now accepted; no tests. |
| `07-extract-pricing-module` | should pass (Refactoring without behavior change, complexity 2) | Clean multi-file refactor: same arithmetic moved to a module, Totals re-exported, new tests. |
