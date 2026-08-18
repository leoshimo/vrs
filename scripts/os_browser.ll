#!/usr/bin/env vrsctl
# os_browser.ll - OS-specific Browser
#

(def current_browser "Safari")

(defn! open_url (url)
  "Open a URL in the configured browser."
  (exec "open" "-a" current_browser url))

(defn! active_tab_safari ()
  "Retrieve the active tab info for Safari"
  (if (not? (eq? (get (exec "pgrep" "-ax" "Safari") :exit) 0))
    nil
    (begin
     (def url_result (exec "osascript" "-e" "tell application \"Safari\" to return URL of front document"))
     (def title_result (exec "osascript" "-e" "tell application \"Safari\" to return name of front document"))
     (def url (get (decode :lines (get url_result :stdout)) 0))
     (def title (get (decode :lines (get title_result :stdout)) 0))
     (list :title title :url url))))

(defn! active_tab_chrome ()
  "Retrieve the active tab info for Chrome"
  (def url_result (exec "osascript" "-e" "tell application \"Google Chrome\" to return URL of active tab of front window"))
  (def title_result (exec "osascript" "-e" "tell application \"Google Chrome\" to return title of active tab of front window"))
  (def url (get (decode :lines (get url_result :stdout)) 0))
  (def title (get (decode :lines (get title_result :stdout)) 0))
  (list :title title :url url))

(defn! active_tab ()
  "(active_tab) Retrieve the current URL of active browser window"
  (match current_browser
    ("Safari" (active_tab_safari))
    ("Google Chrome" (active_tab_chrome))
    (_ (error "Unrecognized browser"))))

(defn! active_tab_for_app (app)
  "Retrieve a tab from the originating browser, not the launcher's current app"
  (match app
    ("Safari" (active_tab_safari))
    ("Google Chrome" (active_tab_chrome))
    (_ nil)))

(defn! read_safari_cloud_tabs (path)
  "Read Safari's synced tab cache, newest last-viewed time first; unknown times sort last."
  (def result
    (exec "sqlite3" "-readonly" "-json" "-cmd" ".timeout 1000" path
      "SELECT t.tab_uuid AS id, t.device_uuid AS device_id,
              coalesce(d.device_name, 'Unknown device') AS device,
              coalesce(nullif(trim(t.title), ''), t.url) AS title, t.url,
              CASE WHEN t.last_viewed_time > 0
                   THEN strftime('%Y-%m-%dT%H:%M:%fZ',
                                 t.last_viewed_time + 978307200, 'unixepoch')
                   ELSE NULL END AS last_viewed_at
         FROM cloud_tabs t
         LEFT JOIN cloud_tab_devices d ON d.device_uuid = t.device_uuid
        WHERE length(trim(t.url)) > 0
        ORDER BY CASE WHEN t.last_viewed_time > 0 THEN t.last_viewed_time END DESC,
                 t.device_uuid, t.tab_uuid"))
  (if (not? (eq? (get result :exit) 0))
    (error (str "Could not read Safari iCloud tabs: " (get result :stderr))))
  (if (eq? (get result :stdout) "") '()
    (map (decode :json (get result :stdout)) (fn (tab) (+ '(:web/page) tab)))))

(defn! cloud_tabs ()
  "(cloud_tabs) - Safari only: read iCloud tabs from this Mac's synced cache, ordered by last viewed (UTC), newest first. Returns :web/page values with id, device_id, device, title, url, and last_viewed_at (nil if unknown). Reading does not force iCloud sync."
  (if (not? (eq? current_browser "Safari"))
    (error "Cloud tabs are currently supported only for Safari"))
  (def path (shell_expand "~/Library/Containers/com.apple.Safari/Data/Library/Safari/CloudTabs.db"))
  (if (not? (eq? (get (exec "test" "-e" path) :exit) 0))
    (set path (shell_expand "~/Library/Safari/CloudTabs.db")))
  (read_safari_cloud_tabs path))

(defn! browser_pages ()
  "Offer the active browser page as a completion candidate"
  (def tab (try (active_tab)))
  (if (list? tab)
    (if (get tab :url) (list (+ '(:web/page) tab)) '())
    '()))

(set_entity_completions :web/page 'browser_pages)

(defn! active_tab_open_wayback ()
  "(active_tab_open_wayback) - Open current active tab in Wayback Machine"
  (def url (get (active_tab) :url))
  # (open_url (format "https://web.archive.org/web/*/{}" url))
  (open_url (format "https://archive.is/{}" url)))

(spawn_srv! :os_browser :interface '(active_tab active_tab_open_wayback active_tab_for_app browser_pages open_url cloud_tabs))
