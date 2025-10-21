#!/usr/bin/env vrsctl
# eden.ll - Eden Workspace Service
#

(bind_srv :os_notify)

(def eden_shim_script (shell_expand "~/proj/vrs/scripts/eden_ls_shim.sh"))

(defn eden_list ()
  "(eden_list) - List available Eden tabs"
  (read (get (exec eden_shim_script) -1)))

(defn eden_open (id)
  "(eden_open ID) - Open Eden tab by ID"
  (exec "/Users/leoshimo/dots/bin/eden" "open" id))

(defn eden_ai (query)
  "(eden_ai QUERY) - Send a request to EDEN AI"
  (exec "/Users/leoshimo/dots/bin/eden" "ai" query))

(spawn_srv :eden :interface '(eden_list eden_open eden_ai))
