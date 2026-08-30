#!/usr/bin/env bash
# Close duplicate automation PRs whose head branches were already deleted.
# Requires a GitHub token with pull_request write access (personal `gh auth login`).
set -euo pipefail

REPO="${1:-MirS0bhan/the-machine}"
KEEP="${KEEP_PR:-}"

echo "Listing open PRs on ${REPO}..."
mapfile -t rows < <(gh pr list --repo "$REPO" --state open --limit 500 \
  --json number,title,headRefName \
  -q '.[] | "\(.number)\t\(.headRefName)\t\(.title)"')

if ((${#rows[@]} == 0)); then
  echo "No open PRs."
  exit 0
fi

closed=0
for row in "${rows[@]}"; do
  num="${row%%$'\t'*}"
  rest="${row#*$'\t'}"
  branch="${rest%%$'\t'*}"
  title="${rest#*$'\t'}"

  if [[ -n "$KEEP" && "$num" == "$KEEP" ]]; then
    echo "KEEP #${num} (${branch})"
    continue
  fi

  if git ls-remote --exit-code origin "refs/heads/${branch}" >/dev/null 2>&1; then
    echo "SKIP #${num} — branch ${branch} still exists"
    continue
  fi

  echo "CLOSE #${num}: ${title}"
  if ! gh pr close "$num" --repo "$REPO" 2>/dev/null; then
    echo "  failed (need personal gh auth with pull_request write)"
  fi
  ((closed++)) || true
done

echo "Done. Closed ${closed} PR(s)."
