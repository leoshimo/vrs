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
  "(reeder_add URL TITLE) - Save a URL to Feedbin Pages"
  (bind_srv :feedbin)
  (feedbin_save url title))

(defn reeder_saved_pages ()
  "(reeder_saved_pages) - Return the 10 most recently saved Feedbin Pages"
  (let ((bound (try (bind_srv :feedbin))))
    (if (err? bound) '() (feedbin_saved_pages))))

(defn reeder_add_active_tab ()
  "(reeder_add_active_tab) - Add current active page of browser to reeder"
  (if (def (:title title :url url) (active_tab))
    (reeder_add url title)))

(spawn_srv :reeder
  :interface '(reeder_refresh_items
               reeder_get_items
               reeder_saved_pages
               reeder_add
               reeder_add_active_tab))
