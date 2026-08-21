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
  (def result
    (exec "sqlite3" "-json" (shell_expand "~/Library/Safari/History.db")
          "SELECT datetime(visit_time + 978307200, 'unixepoch', 'localtime') AS local_visit_time,
                  TRIM(title) AS title,
                  MIN(url) AS url,
                  domain_expansion
             FROM history_visits
             JOIN history_items ON history_visits.history_item = history_items.id
            WHERE LENGTH(TRIM(COALESCE(title, ''))) > 0
              AND LENGTH(TRIM(COALESCE(url, ''))) > 0
              AND LENGTH(url) < 500
              AND url NOT LIKE '%/search%'
              AND url NOT LIKE '%read.amazon.co.jp%'
            GROUP BY title
            ORDER BY visit_time DESC, visit_count_score DESC
            LIMIT 150"))
  (if (eq? (get result :exit) 0)
    (set safari_history (decode :json (get result :stdout)))
    (set safari_history '())))

(spawn_srv :safari_history :interface '(get_safari_history refresh_safari_history))
