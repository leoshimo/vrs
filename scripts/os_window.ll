#!/usr/bin/env vrsctl
# os_window.ll - OS Window Controls
#

(defn get_windows ()
  "(get_windows) - Get all windows"
  (def (:ok res) (exec "./scripts/yabai_window_shim.sh"))
  (read res))

# TODO: Consider dynamic type check - e.g. `islist?` / `isstring?` to accept flexible window selector
(defn focus_window (window_id)
  "(focus_window WINDOW_ID) - Focus window with given ID"
  (exec "yabai" "-m" "window" (str window_id) "--focus"))

(defn yabai_grid (grid_str)
  (try (exec "yabai" "--message" "window" "--grid" grid_str)))

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
  (try (exec "yabai" "--message" "window" "--display" "1"))
  (try (exec "yabai" "--message" "display" "--focus" "1")))

(defn window_to_aux ()
  "(window_to_aux) - Move window to aux display"
  (try (exec "yabai" "--message" "window" "--display" "2"))
  (try (exec "yabai" "--message" "display" "--focus" "2")))

# TODO: Explore embedding shell scripts in Lyric? Macro? How will pipe work?
(defn window_split ()
  "(window_split) - Split currently focused window and last focused window horizontally in display"
  (try (exec "yabai_window_split")))

(defn show_desktop ()
  "(show_desktop) - Show the desktop"
  (try (exec "yabai" "-m" "space" "--toggle" "show-desktop")))

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

