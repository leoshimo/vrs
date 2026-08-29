#!/usr/bin/env vrsctl
# macOS System Appearance Integration
#

(defn is_darkmode ()
  (def result (exec "osascript"
                    "-e" "tell application \"System Events\""
                    "-e" "tell appearance preferences"
                    "-e" "return dark mode"
                    "-e" "end tell"
                    "-e" "end tell"))
  (eq? (get (decode :lines (get result :stdout)) 0) "true"))

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
  (set_darkmode (not? (is_darkmode))))

# Depends on shortcuts
(defn toggle_color_filters ()
  (exec "shortcuts" "run" "color-filters-toggle"))

(defn toggle_quick_shade ()
  (exec "osascript"
        "-e" "tell application \"System Events\""
        "-e" "set isRunning to (exists (processes where name is \"QuickShade\"))"
        "-e" "end tell"
        "-e" "if isRunning then"
        "-e" "tell application \"QuickShade\" to quit"
        "-e" "else"
        "-e" "tell application \"QuickShade\" to activate"
        "-e" "end if")
  :ok)

(spawn_srv :system_appearance :interface '(toggle_darkmode toggle_color_filters toggle_quick_shade))
