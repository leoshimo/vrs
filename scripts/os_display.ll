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

(register_entity_source :os/display 'get_displays)

(defn! list_alternative_resolutions ()
  "List favorite resolutions and the current mode, marked with (current)."
  (def result (exec "hs" "-q" "-c" "display.list_resolutions()"))
  (if (eq? (get result :exit) 0)
    (decode :lines (get result :stdout))
    (error (get result :stderr))))

(defn! select_resolution (desc)
  "(select_resolution DESC) - Select resolution for descriptor"
  (def result (exec "hs" "-q" "-c" (format """
    local screen = hs.screen.mainScreen()
    local mode = screen:availableModes()[{}]
    assert(mode, 'Resolution is no longer available')
    assert(screen:setMode(mode.w, mode.h, mode.scale, mode.freq, mode.depth),
           'Failed to change resolution')
    """ (display (get (split " (current)" desc) 0)))))
  (if (eq? (get result :exit) 0) result
    (error (get result :stderr))))

(spawn_srv! :os_display :interface '(list_alternative_resolutions select_resolution get_displays))
