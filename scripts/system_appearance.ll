#!/usr/bin/env vrsctl
# macOS System Appearance Integration
#

(defn osa_get_darkmode ()
  (def (:ok result) (exec "osascript"
                          "-e" "tell application \"System Events\""
                          "-e" "tell appearance preferences"
                          "-e" "return dark mode"
                          "-e" "end tell"
                          "-e" "end tell"))
  (eq? result "true"))

(defn osa_set_darkmode (dark)
  (exec "osascript"
        "-e" "on run argv"
        "-e" "tell application \"System Events\""
        "-e" "tell appearance preferences"
        "-e" (if dark "set dark mode to true" "set dark mode to false")
        "-e" "end tell"
        "-e" "end tell"
        "-e" "end run")
  :ok)

(def is_dark (osa_get_darkmode))

(defn toggle_darkmode ()
  (set is_dark (not is_dark))
  (osa_set_darkmode is_dark))

(spawn-srv :system_appearance :interface '(toggle_darkmode))
