#!/usr/bin/env vrsctl
# os_browser_demo.ll - OS-specific Browser (Demo)
#

(defn active_tab ()
  "(active_tab) Retrieve the current URL of active browser window"
  (def url_result (exec "osascript" "-e" "tell application \"Safari\" to return URL of front document"))
  (def title_result (exec "osascript" "-e" "tell application \"Safari\" to return name of front document"))
  (def url (get (decode :lines (get url_result :stdout)) 0))
  (def title (get (decode :lines (get title_result :stdout)) 0))
  (list :title title :url url))

(spawn_srv :os_browser :interface '(active_tab))

# DEMO: Test in REPL
