#!/usr/bin/env bash
# Confirms CI and the dependency audit are both green on a commit before it
# gets tagged for release. GitHub Actions has no way for publish.yml to
# `needs:` a job defined in ci.yml or audit.yml (they're separate workflow
# files), so this has to be run by hand as a pre-tag gate instead - see
# RELEASING.md and https://github.com/Sigilweaver/OpenSXRaw/issues/12.
#
# Usage: scripts/check-release-ready.sh [ref]
#   ref defaults to HEAD.
#
# Requires the gh CLI to be authenticated (gh auth status).

set -euo pipefail

for tool in gh jq git; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "FAIL: '$tool' is required but not found on PATH" >&2
        exit 1
    fi
done

ref="${1:-HEAD}"
# Dereference through annotated tags to the commit they point at - plain
# `git rev-parse <tag>` returns the tag object's own SHA, not the commit's,
# for annotated tags, which would never match a workflow run's head SHA.
sha="$(git rev-parse "${ref}^{commit}")"

echo "Checking release readiness for $ref ($sha)..."

ok=1

check_workflow() {
    local workflow="$1"
    local run_json
    run_json="$(gh run list -w "$workflow" -c "$sha" --json status,conclusion,url -L 1)"

    if [ "$(echo "$run_json" | jq 'length')" -eq 0 ]; then
        echo "FAIL: no $workflow run found for $sha"
        ok=0
        return
    fi

    local status conclusion url
    status="$(echo "$run_json" | jq -r '.[0].status')"
    conclusion="$(echo "$run_json" | jq -r '.[0].conclusion')"
    url="$(echo "$run_json" | jq -r '.[0].url')"

    if [ "$status" != "completed" ]; then
        echo "FAIL: latest $workflow run for $sha has status '$status' (not completed) - $url"
        ok=0
        return
    fi

    if [ "$conclusion" != "success" ]; then
        echo "FAIL: latest $workflow run for $sha concluded '$conclusion' (not success) - $url"
        ok=0
        return
    fi

    echo "OK: $workflow passed for $sha - $url"
}

check_workflow "ci.yml"
check_workflow "audit.yml"

if [ "$ok" -eq 1 ]; then
    echo "Release ready: CI and audit are both green on $sha."
    exit 0
else
    echo "Release NOT ready: fix the failures above before tagging $sha."
    exit 1
fi
