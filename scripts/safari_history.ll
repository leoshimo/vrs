#!/usr/bin/env vrsctl
# safari_history.ll - Access Safari History
#

(def safari_history '())
(def safari_history_path (shell_expand "~/Library/Safari/History.db"))

(defn! get_safari_history ()
  "(get_safari_history) - Get the list of items from Safari History"
  safari_history)

(defn! safari_history_url_key (url)
  "Group tracking and section-anchor variants without changing the URL we open."
  (def fragments (split "#" url))
  (def parts (split "?" (get fragments 0)))
  (def base (get parts 0))
  (def host (get (split "/" base) 2))
  (def query (apply join (concat '("?") (slice parts 1))))
  (def parameters (filter (split "&" query) (fn (parameter)
    (def name (get (split "=" parameter) 0))
    (not? (or! (eq? name "")
               (eq? (get (split "utm_" name) 0) "")
               (contains? '("gclid" "fbclid" "msclkid" "gbraid" "wbraid"
                            "gad_source" "gad_campaignid") name)
               (and! (eq? name "pp")
                     (contains? '("www.youtube.com" "youtube.com" "m.youtube.com") host))
               (and! (eq? name "zx")
                     (contains? '("www.google.com" "google.com") host)))))))
  (def fragment (apply join (concat '("#") (slice fragments 1))))
  # Preserve application routes such as #/item/7, #!view, and #view=7.
  # Gmail also uses plain fragments for distinct mailbox views.
  (def route (or! (contains? fragment "/") (contains? fragment "=")
                  (eq? (get (split "!" fragment) 0) "")
                  (eq? host "mail.google.com")))
  (str base
       (if (empty? parameters) "" (str "?" (apply join (concat '("&") parameters))))
       (if (and! route (not? (eq? fragment ""))) (str "#" fragment) "")))

(defn! distinct_safari_history (entries)
  "Keep the newest visit, including its original URL, for each page."
  (def seen '())
  (filter entries (fn (entry)
    (def key (safari_history_url_key (get entry :url)))
    (if (contains? seen key) false
      (begin (set seen (push seen key)) true)))))

(defn! refresh_safari_history ()
  "(refresh_safari_history) - Refresh in-memory Safari History"
  (def result
    (exec "sqlite3" "-readonly" "-json" "-cmd" ".timeout 1000" safari_history_path
          "WITH visits AS (
             SELECT TRIM(title) AS title, url, visit_time, history_visits.id AS visit_id,
                    ROW_NUMBER() OVER (PARTITION BY url ORDER BY visit_time DESC, history_visits.id DESC) AS rank
               FROM history_visits
               JOIN history_items ON history_visits.history_item = history_items.id
              WHERE LENGTH(TRIM(COALESCE(title, ''))) > 0
                AND LENGTH(TRIM(COALESCE(url, ''))) > 0
           )
           SELECT title, url,
                  datetime(visit_time + 978307200, 'unixepoch', 'localtime') AS visited
             FROM visits WHERE rank = 1
            ORDER BY visit_time DESC, visit_id DESC LIMIT 500"))
  (if (eq? (get result :exit) 0)
    (set safari_history (if (eq? (get result :stdout) "") '()
                          (distinct_safari_history (decode :json (get result :stdout)))))
    (error (str "Could not read Safari history: " (get result :stderr))))
  :ok)

(spawn_srv! :safari_history :interface '(get_safari_history refresh_safari_history))
