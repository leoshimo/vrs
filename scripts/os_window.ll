#!/usr/bin/env vrsctl
# os_window.ll - OS Window Controls
#

(defn get_windows ()
  "(get_windows) - Get all windows"
  (def result (exec "yabai" "-m" "query" "--windows"))
  (if (eq? (get result :exit) 0)
    (map (filter (decode :json (get result :stdout))
                 (fn (window) (get window :is-visible)))
         (fn (window) (list :id (get window :id)
                            :app (get window :app)
                            :title (get window :title))))
    '()))

# TODO: Consider dynamic type check - e.g. `islist?` / `isstring?` to accept flexible window selector
(defn focus_window (window_id)
  "(focus_window WINDOW_ID) - Focus window with given ID"
  (exec "yabai" "-m" "window" (str window_id) "--focus"))

(defn yabai_grid (grid_str)
  (exec "yabai" "--message" "window" "--grid" grid_str))

(defn window_fullscreen ()
  "(window_fullscreen) - Fullscreen window"
  (yabai_grid "1:1:0:0:1:1"))

(defn window_center ()
  (yabai_grid "8:8:1:1:6:6"))
  
(defn window_left ()
  "(window_left) - Move window to left half"
  (yabai_grid "1:2:0:0:1:1"))

(defn window_right ()
  "(window_right) - Move window to right half"
  (yabai_grid "1:2:1:0:1:1"))

(defn window_top_right ()
  "(window_top_right) - Move window to top right corner"
  (yabai_grid "2:2:1:0:1:1"))

(defn window_top_left ()
  "(window_top_left) - Move window to top left corner"
  (yabai_grid "2:2:0:0:1:1"))

(defn window_bottom_left ()
  "(window_bottom_left) - Move window to bottom left corner"
  (yabai_grid "2:2:0:1:1:1"))

(defn window_bottom_right ()
  "(window_bottom_right) - Move window to bottom right corner"
  (yabai_grid "2:2:1:1:1:1"))

(defn window_to_main ()
  "(window_to_main) - Move window to main display"
  (exec "yabai" "--message" "window" "--display" "1")
  (exec "yabai" "--message" "display" "--focus" "1"))

(defn window_to_aux ()
  "(window_to_aux) - Move window to aux display"
  (exec "yabai" "--message" "window" "--display" "2")
  (exec "yabai" "--message" "display" "--focus" "2"))

(defn window_split ()
  "(window_split) - Split currently focused window and last focused window horizontally in display"
  (exec "bash" "-seuo" "pipefail"
        :stdin """
        command -v yabai >/dev/null
        command -v jq >/dev/null

        primary_win="$(yabai -m query --windows | jq 'first(.[] | select(."has-focus" == true)).id')"
        aux_win="$(yabai -m query --windows | jq 'first(.[] | select(."has-focus" == false)).id')"

        yabai --message window "$primary_win" --grid "1:2:0:0:1:1"
        yabai --message window "$aux_win" --grid "1:2:1:0:1:1"
        """))

(defn show_desktop ()
  "(show_desktop) - Show the desktop"
  (exec "yabai" "-m" "space" "--toggle" "show-desktop"))

(spawn_srv :os_window
   :interface '(window_fullscreen window_center
                window_left window_right
                window_top_left window_top_right
                window_bottom_left window_bottom_right
                window_to_main window_to_aux
                window_split
                show_desktop
                get_windows
                focus_window))
