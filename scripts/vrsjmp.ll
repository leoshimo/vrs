#!/usr/bin/env vrsctl
# vrsjmp.ll - vrsjmp commandbar
#

# TODO: Move to init.ll w/ supervision tree
(bind_srv :system_appearance)
(bind_srv :bookmarks)
(bind_srv :nl_agent)
(bind_srv :os_screencap)
(bind_srv :todos)

(defn get_items (query)
  "Retrieve items to display"
  (+ (todo_items)
     (favorite_items)
     (get_bookmarks)
     (query_items query)))

(defn make_item (title command)
  "Create an item with TITLE and COMMAND"
  (list :title title :on_click command))

(defn query_items (query)
  "Return a dynamic list of item for current query"
  (if (not? query) '()
      (list
       (make_item "Add TODO"
                  (list 'add_todo query))
       (make_item "Search Perplexity"
                  (list 'open_url (format "http://perplexity.ai/?q={}&copilot=true" query)))
       (make_item "Search Google"
                  (list 'open_url (format "http://google.com/search?q={}" query)))
       (make_item "Just Do It"
                  (list 'do_it query)))))

(defn todo_items ()
  "(todo_items) - Retrieve todo items and create markup for it"
  (map (get_todos)
       (fn (t) (list :title (get t :title)
                     :on_click (list 'set_todos_done_by_id (get t :id))))))

(defn favorite_items ()
  "Returns list of static vrsjmp items"
  (list
   (make_item "Browser" '(open_app "Safari"))
   (make_item "Things" '(open_app "Things3"))
   (make_item "Terminal" '(open_app "Alacritty"))
   (make_item "Messages" '(open_app "Messages"))
   (make_item "Mail" '(open_app "Spark"))
   (make_item "Cal" '(open_app "Notion Calendar"))
   (make_item "Zulip" '(open_app "Zulip"))
   (make_item "X" '(open_url "https://www.x.com"))
   (make_item "GitHub - vrs" '(open_url "https://www.github.com/leoshimo/vrs"))
   (make_item "Downloads" '(open_file "~/Downloads"))
   (make_item "Dropbox" '(open_file "~/Dropbox"))
   (make_item "Kindle" '(open_app "Kindle"))
   (make_item "Send to Kindle" '(open_url "https://www.amazon.com/gp/sendtokindle"))
   (make_item "RC - Presentations" '(open_url "https://presentations.recurse.com"))
   (make_item "AWS Console" '(open_url "http://console.aws.amazon.com"))
   (make_item "ChatGPT" '(open_url "https://chat.openai.com"))
   (make_item "Slack" '(open_app "Slack"))
   (make_item "Soulver" '(open_app "Soulver 3"))
   (make_item "Restart vrsd" '(exec "pkill" "-ax" "vrsd"))
   (make_item "Toggle Darkmode" '(toggle_darkmode))

   (make_item "Bookmarks - Add" '(bookmark_active_tab))
   (make_item "Bookmarks - Clear" '(clear_bookmarks))

   (make_item "Screen Capture" '(start_screencap))
   ))

(defn on_click (item)
  "Handle an on_click payload from item"
  (def cmd (get item :on_click))
  (def res (try (eval cmd)))
  (if (err? res)
    (notify "Encountered error" (format "{}" err))))


(spawn_srv :vrsjmp :interface '(get_items on_click))
