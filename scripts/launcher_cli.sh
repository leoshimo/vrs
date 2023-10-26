#!/usr/bin/env zsh
# launcher.sh - CLI shell for launcher SVC
#

SELECTED=$(vrsctl -c "(call (pid 0) '(:get_items))" \
    | sed -E "s/^\(\(/\(/g" \
    | sed -E "s/\)\)$/\)/g" \
    | sed -E "s/\) \(/\)\n\(/g" \
    | fzf --no-sort --reverse --height=10)

vrsctl -c "(eval (get '$SELECTED :on_click))"
