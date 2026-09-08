#!/usr/bin/env vrsctl
# Run on the Mac paired with the Ditoo. Calls use its configured default device.
# Install ditooctl in ~/.local/bin, then: ditooctl device add desk ADDRESS
# Files passed to ditoo_show belong to this Mac. No shell interpolation is used.

(def ditoo_executable (shell_expand "~/.local/bin/ditooctl"))

(defn! ditoo_invoke (arguments)
  (def result (apply exec (+ (list ditoo_executable "--json") arguments)))
  (if (eq? (get result :exit) 0)
    (decode :json (get result :stdout))
    (error (get result :stderr))))

(defn! ditoo_status ()
  "Read the Ditoo's live display state on its host Mac"
  (ditoo_invoke '("status")))

(defn! ditoo_brightness ()
  "Read the Ditoo's live brightness (0–100)"
  (ditoo_invoke '("brightness")))

(defn! ditoo_set_brightness (percent)
  "Set Ditoo brightness to 0–100 and verify readback"
  (ditoo_invoke (list "brightness" (display percent))))

(defn! ditoo_mode ()
  "Read the Ditoo's reported display mode and settings"
  (ditoo_invoke '("mode")))

(defn! ditoo_set_mode (mode)
  "Select clock, light, gallery, visualizer, custom, or off"
  (ditoo_invoke (list "mode" mode)))

(defn! ditoo_clock_color (rgb)
  "Select the clock with RRGGBB color, preserving other clock settings"
  (ditoo_invoke (list "mode" "clock" "--color" rgb)))

(defn! ditoo_show (path)
  "Show a 16×16 PNG/GIF from a path on the Ditoo's host Mac"
  (ditoo_invoke (list "show" "--" path)))

(defn! ditoo_text (text)
  "Show 1–4 characters on the Ditoo"
  (ditoo_invoke (list "text" "--" text)))

(defn! ditoo_scroll (text)
  "Scroll 1–40 characters; the animation loops after the CLI exits"
  (ditoo_invoke (list "text" "--scroll" "--" text)))

(defn! ditoo_keyboard (action)
  "Toggle keyboard backlight or cycle next/previous; state is not readable"
  (ditoo_invoke (list "keyboard" action)))

(spawn_srv! :ditoo
  :interface '(ditoo_status ditoo_brightness ditoo_set_brightness
               ditoo_mode ditoo_set_mode ditoo_clock_color
               ditoo_show ditoo_text ditoo_scroll ditoo_keyboard))
