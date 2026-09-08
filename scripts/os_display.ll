#!/usr/bin/env vrsctl
# os_display.ll - Display Commands
#

(defn! get_displays ()
  "List displays as typed objects; yabai uses their arrangement index as selector"
  (def result (exec "yabai" "-m" "query" "--displays"))
  (if (eq? (get result :exit) 0)
    (map (decode :json (get result :stdout)) (fn (display)
      `(:os/display :id ,(get display :id) :index ,(get display :index)
        :title ,(format "Display {}" (get display :index)))))
    '()))

(set_entity_completions :os/display 'get_displays)

(defn! list_alternative_resolutions ()
  "(list_other_resolutions) - Lists available resolution except current)"
  (def result (exec "hs" "-q" "-c" "display.list_resolutions()"))
  (if (eq? (get result :exit) 0)
    (decode :lines (get result :stdout))
    '()))

(defn! select_resolution (desc)
  "(select_resolution DESC) - Select resolution for descriptor"
  (exec "hs" "-q" "-c" (format "display.select_resolution(\"{}\")" desc)))

(spawn_srv! :os_display :interface '(list_alternative_resolutions select_resolution get_displays))
