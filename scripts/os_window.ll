#!/usr/bin/env vrsctl
# os_window.ll - OS Window Controls
#

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

(spawn_srv :os_window
   :interface '(window_fullscreen window_center
                window_left window_right
                window_top_left window_top_right
                window_bottom_left window_bottom_right
                window_to_main window_to_aux))

