#!/usr/bin/env zsh
# launcher_cli.sh - CLI shell for vrsjmp SVC
#

set -eu

SELECTED=$(vrsctl -c "(begin (bind_srv :vrsjmp) (get_items \"\"))" \
    | sed -E "s/^\(\(/\(/g" | sed -E "s/\)\)$/\)/g" | sed -E "s/\) \(/\)\n\(/g" \
    | fzf --exact --no-sort --reverse)

vrsctl -c "(begin (bind_srv :vrsjmp) (on_click '$SELECTED))" >/dev/null
