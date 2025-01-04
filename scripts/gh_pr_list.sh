#!/usr/bin/env bash
# gh_pr_list.sh - gh_pr_list shim
#

# TODO: Embedded shell scripts in lyric - escaped escapes
# TODO: JSON to SEXP?

echo "("
gh pr list -R "$WORK_REPO" --json title,url --template '{{range .}}{{ printf "(:title \"%s\" :url \"%s\")\n" .title .url}}{{end}}'
echo ")"
