#!/usr/bin/env vrsctl
# safari_history.ll - Access Safari History
#

# TODO: Timeout block? (timeout DURATION PROC) - would be nice to cap (exec ...) time

(def safari_history '())

(bind_srv :os_notify)

(defn get_safari_history ()
  "(get_safari_history) - Get the list of items from Safari History"
  safari_history)

(defn refresh_safari_history ()
  "(refresh_safari_history) - Refresh in-memory Safari History"
  (def (:ok res) (exec "./scripts/safari_history_shim.tcl"))
  (set safari_history (read res)))

(spawn_srv :safari_history :interface '(get_safari_history refresh_safari_history))
