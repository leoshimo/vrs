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

(spawn_srv! :os_browser :interface '(active_tab active_tab_open_wayback active_tab_for_app browser_pages open_url))
