#!/usr/bin/env bash

set -e

which gh &> /dev/null || {
  echo "This script requires the Github CLI 'gh'"
  exit 1
}

pr="$1"
branch="$2"

[[ -n "${pr}" && -n "${branch}" ]] || {
  echo "usage: $0 <pr> <branch>"
  exit 2
}

title=$(gh pr view "${pr}" --json title --jq '.title')
commits=$(gh pr view "${pr}" --json commits --jq '.commits[].oid')

git fetch -q origin "refs/pull/${pr}/head"
git fetch -q origin "refs/heads/${branch}"
branch_head="$(git rev-parse FETCH_HEAD)"

work_tree="$(git rev-parse --git-dir)/backport-worktree"
[[ -e "${work_tree}" ]] || git worktree add -f "${work_tree}" "${branch_head}"
cd "${work_tree}"

git cherry-pick --quit
git switch -c "backport-${pr}-to-${branch}" "${branch_head}"

echo "Cherry picking commits..."
git cherry-pick -x -S ${commits}

gh pr create --title "[Backport ${branch}] ${title}" --body "Backporting #${pr} to ${branch}" --base "${branch}"