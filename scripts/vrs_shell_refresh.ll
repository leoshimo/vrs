#!/usr/bin/env
# vrs_shell_refresh.ll - Refresh vrs_shell on events

(spawn (fn ()
    (try (kill (find_srv :vrs_shell_refresher)))
    (register :vrs_shell_refresher :overwrite)

    (subscribe :rlist_event)
    (subscribe :todos_event)

    (loop (recv)
    (exec "osascript" "-e"
            "tell application id \"tracesOf.Uebersicht\" to refresh widget id \"vrs_shell-jsx\""))))
