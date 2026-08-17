#!/usr/bin/env vrsctl
# safari_history.ll - Access Safari History
#

(def safari_history '())
(def safari_history_path (shell_expand "~/Library/Safari/History.db"))

(defn get_safari_history ()
  "(get_safari_history) - Get the list of items from Safari History"
  safari_history)

(defn refresh_safari_history ()
  "(refresh_safari_history) - Refresh in-memory Safari History"
  (def result
    (exec "sqlite3" "-readonly" "-json" "-cmd" ".timeout 1000" safari_history_path
          "WITH visits AS (
             SELECT TRIM(title) AS title, url, visit_time,
                    ROW_NUMBER() OVER (PARTITION BY url ORDER BY visit_time DESC, history_visits.id DESC) AS rank
               FROM history_visits
               JOIN history_items ON history_visits.history_item = history_items.id
              WHERE LENGTH(TRIM(COALESCE(title, ''))) > 0
                AND LENGTH(TRIM(COALESCE(url, ''))) > 0
           )
           SELECT title, url,
                  datetime(visit_time + 978307200, 'unixepoch', 'localtime') AS visited
             FROM visits WHERE rank = 1
            ORDER BY visit_time DESC LIMIT 500"))
  (if (eq? (get result :exit) 0)
    (set safari_history (if (eq? (get result :stdout) "") '()
                          (decode :json (get result :stdout))))
    (error (str "Could not read Safari history: " (get result :stderr))))
  :ok)

(spawn_srv! :safari_history :interface '(get_safari_history refresh_safari_history))
