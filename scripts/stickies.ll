#!/usr/bin/env vrsctl
# stickies.ll - Bindings to macOS Stickies scripts
#

(defn stickies_get ()
  "(stickies_open) - Returns open Stickies windows"
  (def result
    (exec "osascript"
          :stdin """
          tell application "System Events"
              tell process "Stickies"
                  set window_titles to title of every window
              end tell
          end tell

          set AppleScript's text item delimiters to linefeed
          return window_titles as text
          """))
  (if (eq? (get result :exit) 0)
    (map (decode :lines (get result :stdout))
         (fn (title) (list :title title)))
    '()))

(defn stickies_open (name)
  "(stickies_open NAME) - Open Stickies with NAME"
  (exec "osascript" "-" "Stickies" name
        :stdin """
        on run argv
            set app_name to item 1 of argv
            set window_title to item 2 of argv

            tell application "System Events"
                tell process app_name
                    set target_window to first window where name is window_title
                    set frontmost to true
                    perform action "AXRaise" of target_window
                end tell
            end tell
        end run
        """))

(spawn_srv :stickies :interface '(stickies_get stickies_open))
