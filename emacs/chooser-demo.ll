# Harmless fixture for chooser-manual-tests.md. All action effects stay in memory.
(def demo_history '())

(defn! demo_items ()
  '((:demo/item :title "Same" :id 1 :note "first" :tags (alpha beta))
    (:demo/item :title "Same" :id 2 :note "line\n\"quoted\"" :tags (gamma delta))))

(defn! demo_places ()
  '((:demo/place :title "Here" :id 10)
    (:demo/place :title "There" :id 20)))

(defn! demo_move (item place)
  "Move demo item"
  (interactive :demo/item :demo/place)
  (set demo_history (push demo_history (list :item item :place place)))
  :moved)

(defn! demo_note (item text)
  "Note demo item"
  (interactive :demo/item :demo/text)
  (set demo_history (push demo_history (list :item item :note text)))
  :noted)

(defn! demo_log () demo_history)

(set_entity_completions :demo/item 'demo_items)
(set_entity_completions :demo/place 'demo_places)
(spawn_srv! :chooser_demo :interface '(demo_items demo_places demo_move demo_note demo_log))
