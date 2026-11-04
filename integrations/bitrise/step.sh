#!/usr/bin/env bash
# Bitrise step entry: installs pr-guard and runs it. Inputs arrive as environment variables (version, mode, policy,
# args); they are never pasted into a command line.
set -uo pipefail
: "${version:?the version input is required}"

bin="$HOME/.local/bin"
curl -fsSL --retry 3 "https://raw.githubusercontent.com/typestrict-lab/pr-guard/v${version}/install.sh" \
  | sh -s -- "$version" --dir "$bin" || exit 2

if [[ -n "${BITRISEIO_GIT_BRANCH_DEST:-}" ]]; then
  git fetch --quiet origin "+refs/heads/$BITRISEIO_GIT_BRANCH_DEST:refs/remotes/origin/$BITRISEIO_GIT_BRANCH_DEST" \
    || echo "pr-guard: could not fetch origin/$BITRISEIO_GIT_BRANCH_DEST" >&2
fi

out="${BITRISE_DEPLOY_DIR:-.}"
cmd=(check --policy "${policy:-.github/pr-guard.yml}")
[[ -n "${mode:-}" ]] && cmd+=(--mode "$mode")
read -r -a extra <<< "${args:-}"

# A test bundle in BITRISE_TEST_DEPLOY_DIR shows on the Tests tab.
tests="${BITRISE_TEST_DEPLOY_DIR:-$out}/pr-guard"
mkdir -p "$tests"
"$bin/pr-guard" "${cmd[@]}" "${extra[@]}" \
  --format junit --output "$tests/pr-guard.xml" \
  --format markdown --output "$out/pr-guard.md" \
  --format text --output -
code=$?
[[ -n "${BITRISE_TEST_DEPLOY_DIR:-}" ]] && printf '{"test-name":"pr-guard"}' > "$tests/test-info.json"

if command -v envman >/dev/null 2>&1; then
  envman add --key PR_GUARD_EXIT_CODE --value "$code"
  envman add --key PR_GUARD_COMMENT_PATH --value "$out/pr-guard.md"
fi
exit "$code"
