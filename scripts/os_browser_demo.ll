#!/usr/bin/env vrsctl
# os_browser_demo.ll - OS-specific Browser (Demo)
#

(defn active_tab ()
  "(active_tab) Retrieve the current URL of active browser window"
  (def (:ok url) (exec "osascript" "-e" "tell application \"Safari\" to return URL of front document"))
  (def (:ok title) (exec "osascript" "-e" "tell application \"Safari\" to return name of front document"))
  (list :title title :url url))

(spawn_srv :os_browser :interface '(active_tab))

# DEMO: Test in REPL
