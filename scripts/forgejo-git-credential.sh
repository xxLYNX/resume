#!/usr/bin/env bash
# Git credential helper for Forgejo (reads fj CLI token store).
set -euo pipefail
op=${1:-}
if [[ "$op" != "get" ]]; then
  exit 0
fi
keys="${XDG_DATA_HOME:-$HOME/.local/share}/forgejo-cli/keys.json"
host="192.168.50.18:3000"
want_host=""
while IFS= read -r line; do
  [[ -z "$line" ]] && break
  case "$line" in
    host=*) want_host="${line#host=}" ;;
  esac
done
if [[ "$want_host" != "$host" ]]; then
  exit 0
fi
if [[ ! -f "$keys" ]]; then
  exit 1
fi
name=$(sed -n 's/.*"name": "\([^"]*\)".*/\1/p' "$keys" | head -1)
token=$(sed -n 's/.*"token": "\([^"]*\)".*/\1/p' "$keys" | head -1)
printf 'username=%s\npassword=%s\n' "$name" "$token"
