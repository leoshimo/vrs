#!/usr/bin/env vrsctl
# os_browser.ll - OS-specific Browser

(def current_browser "Safari")

(defn active_tab_safari ()
  "Retrieve the active tab info for Safari"
  (if (err? (try (exec "pgrep" "-ax" "Safari")))
    nil
    (begin
     (def (:ok url) (exec "osascript" "-e" "tell application \"Safari\" to return URL of front document"))
     (def (:ok title) (exec "osascript" "-e" "tell application \"Safari\" to return name of front document"))
     (list :title title :url url))))

(defn active_tab_chrome ()
  "Retrieve the active tab info for Chrome"
  (def (:ok url) (exec "osascript" "-e" "tell application \"Google Chrome\" to return URL of active tab of front window"))
  (def (:ok title) (exec "osascript" "-e" "tell application \"Google Chrome\" to return title of active tab of front window"))
  (list :title title :url url))

(defn active_tab ()
  "Retrieve the current URL of active browser window"
  (match current_browser
    ("Safari" (active_tab_safari))
    ("Google Chrome" (active_tab_chrome))
    (_ (error "Unrecognized browser"))))

(spawn-srv :os_browser :interface '(active_tab))
