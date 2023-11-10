#!/usr/bin/env zsh
# launcher.sh - CLI shell for launcher SVC
#

set -eu

SELECTED=$(vrsctl -c "(begin (bind-srv :launcher) (get_items))" \
    | sed -E "s/^\(\(/\(/g" \
    | sed -E "s/\)\)$/\)/g" \
    | sed -E "s/\) \(/\)\n\(/g" \
    | fzf --exact --no-sort --reverse)

vrsctl -c "(eval (get '$SELECTED :on_click))" >/dev/null
