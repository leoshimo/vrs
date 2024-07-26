#!/usr/bin/env vrsctl
# vrsjmp.ll - vrsjmp commandbar
#

(defn is_personal? ()
  (eq? (exec "uname" "-n") "shinjuku.local"))

# TODO: Move to init.ll w/ supervision tree
(bind_srv :system_appearance)
(bind_srv :rlist)
(bind_srv :nl_shell)
(bind_srv :nl_scheduler)
(bind_srv :os_screencap)
(bind_srv :todos)
(bind_srv :os_window)
(bind_srv :os_maps)

(defn get_items (query)
  "Retrieve items to display"
  (+ (favorite_items)
     (todo_items)
     (window_items query)
     (scheduler_items query)
     (rlist_items query)
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
       (make_item "Search Maps"
                  (list 'open_maps_search query))
       (make_item "Add Task"
                  (list 'add_todo query))
       (make_item "Open App"
                  (list 'open_app query))
       (make_item "Open URL"
                  (list 'open_url query))
       (make_item "Do It"
                  (list 'codegen_exec query))
       (make_item "Force Quit"
                  (list 'exec "pkill" query))
       )))

# TODO: Idea: Window Selector w/ `yabai -m query --windows` -> List of Windows -> Change Focus?
(defn window_items (query)
  "Return item for window commands"
  # Only match if query contains win
  (if (not? (contains? query "win"))
        '()
      (list
       (make_item "Window - Split" '(window_split))
       (make_item "Window - Fullscreen" '(window_fullscreen))
       (make_item "Window - Center" '(window_center))
       (make_item "Window - Left" '(window_left))
       (make_item "Window - Right" '(window_right))
       (make_item "Window - Top Left" '(window_top_left))
       (make_item "Window - Top Right" '(window_top_right))
       (make_item "Window - Bottom Left" '(window_bottom_left))
       (make_item "Window - Bottom Right" '(window_bottom_right))
       (make_item "Window - Main Display" '(window_to_main))
       (make_item "Window - Aux Display" '(window_to_aux))
    )))

(defn scheduler_items (query)
  "Return item for scheduler commands"
  # Only match if query contains win
  (if (not? (contains? query "schedule"))
        '()
      (list
       (make_item "Schedule - Tomorrow" '(schedule_the_day "tomorrow"))
       (make_item "Schedule - Today" '(schedule_the_day "today")))))


(defn todo_items ()
  "(todo_items) - Retrieve todo items and create markup for it"
  (map (get_todos)
       (fn (t) (list :title (format "Mark Done - {}" (get t :title))
                     :on_click (list 'set_todos_done_by_id (get t :id))))))

(defn rlist_items (query)
  "(rlist_items QUERY) - Retrieve item markup for reading list"
  (def items '())
  (map (get_rlist) (lambda (it) (begin
       (set items (push items (list :title (format "rl: Open {}" (get it :title))
                                    :on_click (list 'open_url (get it :url)))))
       # TODO: Plumb "modifiers" from clients?
       (if (contains? query "rl:")
         (set items (push items (list :title (format "rl: Remove {}" (get it :title))
                                      :on_click (list 'remove_rlist (get it :id)))))))))
  items)

(defn favorite_items ()
  "Returns list of static vrsjmp items"
  (+
   # app launcher
   (list (make_item "Browser" '(open_app "Safari"))
         (make_item "Terminal" '(open_app "Ghostty")) # 👻
         # (make_item "Terminal" '(open_app "Alacritty"))
         (make_item "Things" '(open_app "Things3"))
         (make_item "Messages" '(open_app "Messages"))
         (make_item "Notes" '(open_app "Notes"))
         (make_item "Reminders" '(open_app "Reminders"))
         # (make_item "Mail" '(open_app "Spark"))
         (make_item "Mail" '(open_app "Mimestream"))
         (make_item "Cal" '(open_app "Notion Calendar"))
         (make_item "Slack" '(open_app "Slack"))
         (make_item "Soulver" '(open_app "Soulver 3"))
         (make_item "1Password" '(open_app "1Password"))
         (make_item "Excalidraw" '(open_url "https://excalidraw.com"))
         (make_item "XCode" '(exec "open_xcode")) # TODO: Built-in regex
         (make_item "Chrome" '(open_app "Google Chrome")))


   # directories
   (list (make_item "Downloads" '(open_file "~/Downloads"))
         (make_item "Dropbox" '(open_file "~/Dropbox"))
         (make_item "Crash Reports" '(open_file "~/Library/Logs/DiagnosticReports/")))

   # links
   (list (make_item "GitHub - vrs" '(open_url "https://www.github.com/leoshimo/vrs"))
         (make_item "X" '(open_url "https://www.x.com"))
         (make_item "Send to Kindle" '(open_url "https://www.amazon.com/gp/sendtokindle"))
         (make_item "Claude.ai" '(open_url "https://claude.ai/new"))
         # (make_item "ChatGPT" '(open_url "https://chat.openai.com"))
         (make_item "Are.na" '(open_url "https://www.are.na/leo-shimo/moodboard-fiffzxstqdq"))
         (make_item "Tiktokenizer" '(open_url "https://tiktokenizer.vercel.app"))
         (make_item "CyberChef" '(open_url "https://gchq.github.io/CyberChef/")))

   # apps - personal
   (if (is_personal?)
     (list (make_item "Zulip" '(open_app "Zulip"))
           (make_item "Kindle" '(open_app "Kindle"))
           (make_item "AWS Console" '(open_url "http://console.aws.amazon.com")))
     '())

   # machine-local
   (local_items)

   # misc
   (list (make_item "Restart vrsd" '(exec "pkill" "-ax" "vrsd"))
         (make_item "Toggle Darkmode" '(toggle_darkmode))
         (make_item "Toggle Color Filter" '(toggle_color_filters))
         (make_item "Open in Wayback" '(active_tab_open_wayback))
         (make_item "Show Desktop" '(show_desktop))
         (make_item "Toggle DND" '(toggle_do_not_disturb)))

   # reading
   (list (make_item "Add to Reading List" '(add_rlist_active_tab))
         (make_item "Clear Reading List" '(clear_rlist)))

   # recording
   (list (make_item "Screen Capture" '(start_screencap)))
   ))

(defn local_items ()
  "Read set of local items if any"
  (def res (try (fread "~/vrsjmp_local.ll")))
  (if (ok? res) res '()))

(defn on_click (item)
  "Handle an on_click payload from item"
  (def cmd (get item :on_click))
  (publish :cmd cmd)
  (spawn (fn ()
           (def res (try (eval cmd)))
           (if (err? res)
             (notify "Encountered error" (display res))))))

(spawn_srv :vrsjmp :interface '(get_items on_click))

