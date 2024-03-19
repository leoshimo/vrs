#!/usr/bin/env vrsctl
# vrsjmp.ll - vrsjmp commandbar
#

(defn is_personal? ()
  (eq? (exec "uname" "-n") "shinjuku.local"))

# TODO: Move to init.ll w/ supervision tree
(bind_srv :system_appearance)
(bind_srv :rlist)
(bind_srv :nl_shell)
(bind_srv :os_screencap)
(bind_srv :todos)

(defn get_items (query)
  "Retrieve items to display"
  (+ (favorite_items)
     (todo_items)
     (rlist_items)
     (query_items query)))

(defn make_item (title command)
  "Create an item with TITLE and COMMAND"
  (list :title title :on_click command))

(defn query_items (query)
  "Return a dynamic list of item for current query"
  (if (not? query) '()
      (list
       (make_item "Search Perplexity"
                  (list 'open_url (format "http://perplexity.ai/?q={}&copilot=true" query)))
       (make_item "Search Google"
                  (list 'open_url (format "http://google.com/search?q={}" query)))
       (make_item "Add Task"
                  (list 'add_todo query))
       (make_item "Open App"
                  (list 'open_app query))
       (make_item "Do It"
                  (list 'do_it query)))))

(defn todo_items ()
  "(todo_items) - Retrieve todo items and create markup for it"
  (map (get_todos)
       (fn (t) (list :title (format "Mark Done - {}" (get t :title))
                     :on_click (list 'set_todos_done_by_id (get t :id))))))

(defn rlist_items ()
  "(rlist_items) - Retrieve item markup for reading list"
  (map (get_rlist)
       (fn (b) (list :title (format "Reading List - {}" (get b :title))
                     :on_click (list 'open_url (get b :url))))))

(defn favorite_items ()
  "Returns list of static vrsjmp items"
  (+
   # apps
   (list (make_item "Browser" '(open_app "Safari"))
         (make_item "Things" '(open_app "Things3"))
         (make_item "Terminal" '(open_app "Alacritty"))
         (make_item "Messages" '(open_app "Messages"))
         (make_item "Mail" '(open_app "Spark"))
         (make_item "Cal" '(open_app "Notion Calendar"))
         (make_item "Slack" '(open_app "Slack"))
         (make_item "Soulver" '(open_app "Soulver 3"))
         (make_item "1Password" '(open_app "1Password"))
         (make_item "Chrome" '(open_app "Google Chrome")))

   # directories
   (list (make_item "Downloads" '(open_file "~/Downloads"))
         (make_item "Dropbox" '(open_file "~/Dropbox")))

   # links
   (list (make_item "GitHub - vrs" '(open_url "https://www.github.com/leoshimo/vrs"))
         (make_item "X" '(open_url "https://www.x.com"))
         (make_item "Send to Kindle" '(open_url "https://www.amazon.com/gp/sendtokindle"))
         (make_item "ChatGPT" '(open_url "https://chat.openai.com")))

   # apps - personal
   (if (is_personal?)
     (list (make_item "Zulip" '(open_app "Zulip"))
           (make_item "Kindle" '(open_app "Kindle"))
           (make_item "AWS Console" '(open_url "http://console.aws.amazon.com")))
     '())

   # apps - nonpersonal
   (if (not? (is_personal?))
     (list (make_item "Linear" '(open_app "Linear"))
           (make_item "Notion" '(open_app "Notion"))
           (make_item "iCloud Drive - SAI" '(open_file "~/Library/Mobile\ Documents/com\~apple\~CloudDocs/Software\ Applications\ Incorporated")))
     '())

   # misc
   (list (make_item "Restart vrsd" '(exec "pkill" "-ax" "vrsd"))
         (make_item "Toggle Darkmode" '(toggle_darkmode)))

   # reading
   (list (make_item "Add to Reading List" '(add_rlist_active_tab))
         (make_item "Clear Reading List" '(clear_rlist)))

   # recording
   (list (make_item "Screen Capture" '(start_screencap)))
   ))

(defn on_click (item)
  "Handle an on_click payload from item"
  (def cmd (get item :on_click))
  (def res (try (eval cmd)))
  (if (err? res)
    (notify "Encountered error" (format "{}" err))))


(spawn_srv :vrsjmp :interface '(get_items on_click))
