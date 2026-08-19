#!/usr/bin/env zsh
# CLI palette: navigate pages, locally filter each page's initial batch with fzf.
# Use the GUI for live server-side search and debounce (e.g. all Read Later pages).
set -eu

command -v jq >/dev/null
command -v fzf >/dev/null
typeset -a pages ctl
ctl=(vrsctl "$@")
pages=("$("${ctl[@]}" -c '(begin (bind_srv :vrsjmp) (root_page))')")

while (( ${#pages} )); do
    page=${pages[-1]}
    # Render one serialized item per line. jq decodes only the outer string;
    # quotes, newlines, and nested object arguments inside items stay escaped.
    items=$("${ctl[@]}" -c "(begin (bind_srv :vrsjmp)
      (def page '$page)
      (def args (get page :args))
      (apply join (+ '(\"\n\") (map
        (get_items (get page :get_items) (if args args '()) \"\") display))))" | jq -r .)
    if selected=$(print -r -- "$items" | fzf --exact --no-sort --reverse); then
        result=$("${ctl[@]}" -c "(begin (bind_srv :vrsjmp) (on_click '$selected))")
        if [[ "$result" == ':close' ]]; then exit 0; fi
        pages+=("$result")
    else
        pages[-1]=()
    fi
done
