#!/usr/bin/env vrsctl
# macOS System Appearance Integration
#

(defn is_darkmode ()
  (def (:ok result) (exec "osascript"
                          "-e" "tell application \"System Events\""
                          "-e" "tell appearance preferences"
                          "-e" "return dark mode"
                          "-e" "end tell"
                          "-e" "end tell"))
  (eq? result "true"))

(defn set_darkmode (dark)
  (exec "osascript"
        "-e" "on run argv"
        "-e" "tell application \"System Events\""
        "-e" "tell appearance preferences"
        "-e" (if dark "set dark mode to true" "set dark mode to false")
        "-e" "end tell"
        "-e" "end tell"
        "-e" "end run")
  :ok)

(defn toggle_darkmode ()
  (set_darkmode (not (is_darkmode))))

(spawn_srv :system_appearance :interface '(toggle_darkmode))
