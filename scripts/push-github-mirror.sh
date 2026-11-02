#!/usr/bin/env bash
# Push current branch to the GitHub mirror remote (after Forgejo is primary).
set -euo pipefail
branch="${1:-master}"
if ! git remote get-url github &>/dev/null; then
  echo "No 'github' remote. Add with: git remote add github git@github.com:xxLYNX/resume.git" >&2
  exit 1
fi
git push github "HEAD:refs/heads/${branch}"
