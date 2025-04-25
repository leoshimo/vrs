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
(bind_srv :os_display)
(bind_srv :os_window)
(bind_srv :os_maps)
(bind_srv :os_notes)
(bind_srv :obsidian)
(bind_srv :youtube)
(bind_srv :cmd_macro)
(bind_srv :safari_history)
(bind_srv :github)
(bind_srv :reeder)

(defn get_items (query)
  "Retrieve items to display"
  (+ (favorite_items)
     (todo_items query)
     (notes_items query)
     (obsidian_items query)
     (display_items query)
     (window_items query)
     (scheduler_items query)
     # (reeder_items query)
     (rlist_items query)
     (youtube_items query)
     (safari_history_items query)
     (github_items query)
     (macro_items query)
     (query_items query)))

(defn make_item (title command)
  "Create an item with TITLE and COMMAND"
  (list :title title :on_click command))

(defn make_item_ex (title command hints)
  "Create an item with TITLE and COMMAND and HINTS"
  (list :hints hints :title title :on_click command))

(defn open_claude ()
  "Open Claude.app from correct directory"
  (if (err? (try (exec "pgrep" "-x" "Claude")))
    (begin
     (spawn (lambda () (exec "/Applications/Claude.app/Contents/MacOS/Claude")))
     (sleep 1)))
  (open_app "Claude"))

# TODO: Query should be rule-based? I.e. "Search DWIM" - if URL, if App Name, if Bundle ID, if location (?), if long, etc
(defn query_items (query)
  "Return a dynamic list of item for current query"
  (if (not? query) '()
      (list
       (make_item "Search Google"
                  (list 'open_url (format "http://google.com/search?q={}" query)))
       (make_item "Search Maps"
                  (list 'open_maps_search query))
       (make_item "Search Perplexity"
                  (list 'open_url (format "http://perplexity.ai/?q={}&copilot=true" query)))
       (make_item "Search YT Music"
                  (list 'open_url (format "http://music.youtube.com/search?q={}" query)))
       (make_item "Add Todo"
                  (list 'add_todo query))
       (make_item "Open App"
                  (list 'open_app query))
       (make_item "Open URL"
                  (list 'open_url query))
       (make_item "Do It"
                  (list 'codegen_exec query))
       (make_item "Force Quit"
                  (list 'exec "pkill" query))
       (make_item "Search Amazon"
                  (list 'open_url (format "https://www.amazon.com/s?k={}" query)))
       )))

(defn display_items (query)
  "Return item for display commands"
  (if (not? (contains? query "d:"))
    '()
    (map (list_alternative_resolutions) (fn (r) (make_item (format "d: {}" r) (list 'select_resolution r))))))

(defn window_items (query)
  "Return item for window commands"
  (if (not? (contains? query "w:"))
    '()
      (+
       (map (get_windows)
            (fn (w) (make_item (format "w: {} - {}" (get w :app) (get w :title))
                               (list 'focus_window (get w :id)))))
       (list
        (make_item "w: Split" '(window_split))
        (make_item "w: Fullscreen" '(window_fullscreen))
        (make_item "w: Center" '(window_center))
        (make_item "w: Left" '(window_left))
        (make_item "w: Right" '(window_right))
        (make_item "w: Top Left" '(window_top_left))
        (make_item "w: Top Right" '(window_top_right))
        (make_item "w: Bottom Left" '(window_bottom_left))
        (make_item "w: Bottom Right" '(window_bottom_right))
        (make_item "w: Main Display" '(window_to_main))
        (make_item "w: Aux Display" '(window_to_aux)))
       )))

(defn scheduler_items (query)
  "Return item for scheduler commands"
  # Only match if query contains win
  (if (not? (contains? query "schedule"))
        '()
      (list
       (make_item "Schedule - Tomorrow" '(schedule_the_day "tomorrow"))
       (make_item "Schedule - Today" '(schedule_the_day "today")))))


(defn todo_items (query)
  "(todo_items) - Retrieve todo items and create markup for it"
  (if (not? (contains? query "t: "))
    '()
      (map (get_todos)
           (fn (t) (list :title (format "t: Mark Done - {}" (get t :title))
                         :on_click (list 'set_todos_done_by_id (get t :id)))))))

# (defn reeder_items (query)
#   "(reeder_items QUERY) - Returns markup for reeder items"
#   (if (not? (contains? query "rd:"))
#       '()
#       (begin
#        (if (eq? query "rd:") (reeder_refresh_items))
#        (map (reeder_get_items) (fn (it) (make_item (format "rd: {}" (get it :title)) (list 'open_url (get it :url))))))))

(defn notes_items (query)
  "(notes_items) - Returns markup for notes"
  (if (not? (contains? query "n:"))
    '()
    (map (get_notes) (fn (n) (list :title (format "n: {}" (get n :title))
                                   :on_click (list 'open_note (get n :id)))))))

(defn obsidian_items (query)
  "(obsidian_items) - Returns markup for obsidian notes"
  (if (not? (contains? query "o:"))
    '()
    (map (get_obsidian_files) (fn (n) (list :title (format "o: {}" (get n :title))
                                            :on_click (list 'open_obsidian_file (get n :file)))))))

(defn youtube_items (query)
  "(youtube_items QUERY) - Returns markup for youtube items"
  (if (not? (contains? query "yt:"))
    (list
     (make_item "Download YT Video" '(download_video_active_tab)))
    (map (list_videos) (fn (n) (list :title (format "yt: {}" (get n :title))
                                     :on_click (list 'open_file (get n :path)))))))

# TODO: Nice-to-have - "subtitle" UI to show url / domain
(defn safari_history_items (query)
  "(safari_history_items QUERY) - Returns markup for safari history items"
  (if (not? (contains? query "h:"))
      '()
      (begin
       (if (eq? query "h:") (refresh_safari_history)) # refresh on "appear"
       (map (get_safari_history) (fn (h) (make_item_ex (format "h: {}" (get h :title))
                                                       (list 'open_url (get h :url))
                                                       (get h :domain_expansion)))))))

(defn github_items (query)
  "(github_items QUERY) - Returns markup for github items"
  (if (not? (contains? query "gh:"))
      '()
      (begin
       (if (eq? query "gh:") (refresh_pull_requests))
       (map (get_pull_requests) (fn (pr) (make_item (format "gh: {}" (get pr :title)) (list 'open_url (get pr :url))))))))

# TODO: Nice to have "prefix-drop" for these prefixed names
(defn macro_items (query)
  "(macro_items QUERY) - Returns markup for macro items"
  (if (not? (contains? query "macro:"))
    '()
    (+
     (map (get_macros) (fn (m) (list :title (get m :name)
                                     :on_click (list 'eval (get m :cmds)))))
     (list
      (if (macro_is_recording)
        (make_item "macro: Stop Recording" '(end_macro_record))
        (make_item (format "macro: Start Recording - {}" query) (list 'start_macro_record query)))
      (make_item "macro: Clear Macros" '(clear_macros))
      ))))

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
   (list

         (make_item_ex "Browser" '(open_app "Safari") 'b)
         (make_item_ex "Deta Surf" '(open_app "Surf") 'b)
         (make_item_ex "Terminal" '(open_app "Ghostty") 't) # 👻
         (make_item_ex "TextEdit" '(open_app "TextEdit") 'te)
         # (make_item "Terminal" '(open_app "Alacritty"))
         (make_item "Things" '(open_app "Things3"))
         (make_item "Messages" '(open_app "Messages"))
         (make_item "Notes" '(open_app "Notes"))
         (make_item "Reminders" '(open_app "Reminders"))
         (make_item "Shortcuts" '(open_app "Shortcuts"))
         # (make_item "Mail" '(open_app "Spark"))
         (make_item "Mail" '(open_app "Mimestream"))
         (make_item "Cal" '(open_app "Notion Calendar"))
         (make_item "Find My" '(open_app "FindMy"))
         (make_item "Slack" '(open_app "Slack"))
         (make_item "Soulver" '(open_app "Soulver 3"))
         (make_item "1Password" '(open_app "1Password"))
         (make_item "TLDraw" '(open_url "https://www.tldraw.com"))
         (make_item "Xcode" '(exec "open_xcode")) # TODO: Built-in regex
         (make_item "Chrome" '(open_app "Google Chrome"))
         (make_item "Obsidian" '(open_app "Obsidian"))
         (make_item "Script Debugger" '(open_app "Script Debugger"))
         (make_item "ProxyMan" '(open_app "ProxyMan"))
         (make_item_ex "Reeder" '(open_app "Reeder") 'reeder)
         (make_item "Feedbin" '(open_url "https://feedbin.com/"))

         (make_item "UI Browser" '(open_app "UI Browser"))

         # Assistants
         (make_item "Claude" '(open_claude))
         (make_item "ChatGPT" '(open_app "ChatGPT"))
         (make_item "HuggingChat" '(open_app "HuggingChat"))

         (make_item "Zig - langref" '(open_file "~/.zigup/doc/langref.html")))

   # directories
   (list
         (make_item_ex "Desktop" '(open_file "~/Desktop") 'desktop)
         (make_item_ex "Downloads" '(open_file "~/Downloads") 'download)
         (make_item "iCloud Drive" '(open_file "~/Library/Mobile Documents/com~apple~CloudDocs"))
         (make_item "Dropbox" '(open_file "~/Dropbox"))
         (make_item "Crash Reports" '(open_file "~/Library/Logs/DiagnosticReports/")))

   # links
   (list (make_item "GitHub - vrs" '(open_url "https://www.github.com/leoshimo/vrs"))
         (make_item "X" '(open_url "https://www.x.com"))
         (make_item "Send to Kindle" '(open_url "https://www.amazon.com/gp/sendtokindle"))
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
         (make_item "Toggle Desktop" '(exec "desktop_toggle"))
         (make_item "Toggle QuickShade" '(toggle_quick_shade))
         (make_item "Open in Wayback" '(active_tab_open_wayback))
         (make_item "Show Desktop" '(show_desktop))
         (make_item "Toggle DND" '(toggle_do_not_disturb)))

   # reeder
   (list (make_item "Add to Reeder" '(reeder_add_active_tab)))

   # jump list
   (list (make_item "Add to Jump List" '(add_rlist_active_tab))
         (make_item "Clear Jump List" '(clear_rlist)))

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
