#!/usr/bin/env vrsctl
# stickies.ll - Bindings to macOS Stickies scripts
#

(defn stickies_get ()
  "(stickies_open) - Returns open Stickies windows"
  (def (:ok res) (exec "./scripts/stickies_get.sh"))
  (read res))

(defn stickies_open (name)
  "(stickies_open NAME) - Open Stickies with NAME"
  (exec "./scripts/ax_raise.sh" "Stickies" name))

(spawn_srv :stickies :interface '(stickies_get stickies_open))
  
