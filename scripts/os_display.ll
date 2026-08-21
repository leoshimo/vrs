#!/usr/bin/env vrsctl
# os_display.ll - Display Commands
#

(defn list_alternative_resolutions ()
  "(list_other_resolutions) - Lists available resolution except current)"
  (def result (exec "hs" "-q" "-c" "display.list_resolutions()"))
  (if (eq? (get result :exit) 0)
    (decode :lines (get result :stdout))
    '()))

(defn select_resolution (desc)
  "(select_resolution DESC) - Select resolution for descriptor"
  (exec "hs" "-q" "-c" (format "display.select_resolution(\"{}\")" desc)))

(spawn_srv :os_display :interface '(list_alternative_resolutions select_resolution))
