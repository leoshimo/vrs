#!/usr/bin/env vrsctl
# stickies.ll - Bindings to macOS Stickies scripts
#

(defn stickies_get ()
  "(stickies_open) - Returns open Stickies windows"
  (def result
    (exec "osascript"
          "-e" "tell application \"System Events\""
          "-e" "tell process \"Stickies\""
          "-e" "set window_titles to title of every window"
          "-e" "end tell"
          "-e" "end tell"
          "-e" "set AppleScript's text item delimiters to linefeed"
          "-e" "return window_titles as text"))
  (if (eq? (get result :exit) 0)
    (map (decode :lines (get result :stdout))
         (fn (title) (list :title title)))
    '()))

(defn stickies_open (name)
  "(stickies_open NAME) - Open Stickies with NAME"
  (exec "./scripts/ax_raise.sh" "Stickies" name))

(spawn_srv :stickies :interface '(stickies_get stickies_open))
  
