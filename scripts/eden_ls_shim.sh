#!/usr/bin/env bash
# eden_ls_shim - Shim Bash Script
#

set -euo pipefail

escape_for_sexpr() {
    local value="$1"
    value=${value//\\/\\\\}
    value=${value//\"/\\\"}
    printf '%s' "$value"
}

if ! eden_output=$(eden ls 2>/dev/null); then
    echo "()"
    exit 0
fi

echo "("
while IFS=$'\t' read -r id title || [[ -n "${id-}" ]]; do
    [[ -z "${id}" ]] && continue
    escaped_id=$(escape_for_sexpr "$id")
    escaped_title=$(escape_for_sexpr "${title-}")
    printf '(:id "%s" :title "%s")\n' "$escaped_id" "$escaped_title"
done <<<"$eden_output"
echo ")"
