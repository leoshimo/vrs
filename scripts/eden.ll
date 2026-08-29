#!/usr/bin/env vrsctl
# eden.ll - Eden Workspace Service
#

(bind_srv :os_notify)

(defn eden_list ()
  "(eden_list) - List available Eden tabs"
  (def result (exec "eden" "ls"))
  (if (eq? (get result :exit) 0)
    (decode :tsv (get result :stdout) :columns '(:id :title))
    '()))

(defn eden_open (id)
  "(eden_open ID) - Open Eden tab by ID"
  (exec "eden" "open" id))

(defn eden_ai (query)
  "(eden_ai QUERY) - Send a request to EDEN AI"
  (exec "eden" "ai" query))

(spawn_srv :eden :interface '(eden_list eden_open eden_ai))
