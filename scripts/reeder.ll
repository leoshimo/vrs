#!/usr/bin/env vrsctl
# reeder.ll - Reeder integration
#

(bind_srv :os_browser)

# v hacky wiring via shortcuts over shell

(def items '())

(defn reeder_refresh_items ()
  "(reeder_refresh_items) - Refresh items from Reeder"
  (def (:ok res) (exec "./scripts/reeder_refresh_shim.sh"))
  (set items (read res))
  :ok)

(defn reeder_get_items ()
  "(reeder_get_items) - Return all unread items in reeder"
  items)

(defn reeder_add (url)
  "(reeder_add URL) - Add item with URL to Reeder"
  (exec "bash" "-c" (format "echo \"{}\" | shortcuts run \"add-to-reeder\"" url))
  :ok)

(defn reeder_add_active_tab ()
  "(reeder_add_active_tab) - Add current active page of browser to reeder"
  (if (def (:title title :url url) (active_tab))
    (reeder_add url)))

(spawn_srv :reeder :interface '(reeder_refresh_items reeder_get_items reeder_add reeder_add_active_tab))
