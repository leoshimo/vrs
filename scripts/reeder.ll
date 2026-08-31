#!/usr/bin/env vrsctl
# reeder.ll - Reeder integration
#

(bind_srv :os_browser)

(def items '())

(defn reeder_refresh_items ()
  "(reeder_refresh_items) - Refresh items from Reeder"
  (def result (exec "shortcuts" "run" "get-unread-reeder"))
  (if (eq? (get result :exit) 0)
    (if (empty? (decode :lines (get result :stdout)))
      (set items '())
      (set items (decode :json (get result :stdout))))
    (error (get result :stderr)))
  :ok)

(defn reeder_get_items ()
  "(reeder_get_items) - Return all unread items in reeder"
  items)

(defn reeder_add (url title)
  "(reeder_add URL title) - Add item with URL to Reeder"
  (def result
    (exec "bash" "-seuo" "pipefail" "--" url title
          :stdin """
          basic_auth="$(op item get Feedbin --fields 'label=basic_auth')"

          curl --silent \
               --request POST \
               --user "$basic_auth" \
               --data-urlencode "url=$1" \
               --data-urlencode "title=$2" \
               https://api.feedbin.com/v2/pages.json
          """))
  (if (eq? (get result :exit) 0)
    :ok
    (error (get result :stderr))))

(defn reeder_add_active_tab ()
  "(reeder_add_active_tab) - Add current active page of browser to reeder"
  (if (def (:title title :url url) (active_tab))
    (reeder_add url title)))

(spawn_srv :reeder :interface '(reeder_refresh_items reeder_get_items reeder_add reeder_add_active_tab))
