#!/usr/bin/env vrsctl
# os_display.ll - Display Commands
#

(defn list_alternative_resolutions ()
  "(list_other_resolutions) - Lists available resolution except current)"
  (def (:ok res) (exec "hs" "-q" "-c" "display.list_resolutions()"))
  (split "\n" res))

(defn select_resolution (desc)
  "(select_resolution DESC) - Select resolution for descriptor"
  (exec "hs" "-q" "-c" (format "display.select_resolution(\"{}\")" desc)))

(spawn_srv :os_display :interface '(list_alternative_resolutions select_resolution))

