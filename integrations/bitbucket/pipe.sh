#!/usr/bin/env bash
# Bitbucket Pipe entrypoint. Variables: MODE (shadow | enforce, empty keeps the policy's), POLICY, ARGS, DEBUG.
# The pipe runs in the clone ($BITBUCKET_CLONE_DIR); it reads the diff and never runs code from the pull request.
set -uo pipefail

cd "${BITBUCKET_CLONE_DIR:-.}" || exit 2
[[ "${DEBUG:-false}" == "true" ]] && set -x

# The merge base needs the destination branch; clones are shallow unless the step sets `clone: depth: full`.
if [[ -n "${BITBUCKET_PR_DESTINATION_BRANCH:-}" ]]; then
  git fetch --quiet origin "+refs/heads/$BITBUCKET_PR_DESTINATION_BRANCH:refs/remotes/origin/$BITBUCKET_PR_DESTINATION_BRANCH" \
    || echo "pr-guard: could not fetch origin/$BITBUCKET_PR_DESTINATION_BRANCH; set clone depth: full" >&2
fi

args=(check --policy "${POLICY:-.github/pr-guard.yml}")
[[ -n "${MODE:-}" ]] && args+=(--mode "$MODE")
# ARGS is a list of flags on purpose.
read -r -a extra <<< "${ARGS:-}"

# test-results/ is where Pipelines looks for JUnit reports; the markdown is the sticky comment's text.
mkdir -p test-results
pr-guard "${args[@]}" "${extra[@]}" \
  --format junit --output test-results/pr-guard.xml \
  --format markdown --output pr-guard.md \
  --format text --output -
