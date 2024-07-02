#!/usr/bin/env vrsctl
# os_maps - Maps
#

(defn open_maps_search (query)
  "Start maps search for given query"
  (exec "open" (format "maps://?q={}" query)))

(spawn_srv :os_maps :interface '(open_maps_search))
