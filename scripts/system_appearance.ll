#!/usr/bin/env vrsctl
# System Appearance Integration - Requires `darkmode` script 
#      darkmode - https://github.com/leoshimo/dots/blob/ad657576c94d2320abf089a4521f4c2c31640b34/bin/darkmode
#

(def is_dark (eq? (get (exec "darkmode") 1) "true"))

(defn toggle_darkmode ()
  (set is_dark (not is_dark))
  (exec "darkmode" (if is_dark "true" "false")))

(spawn-srv :system_appearance :interface '(toggle_darkmode))
