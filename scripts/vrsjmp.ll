#!/usr/bin/env vrsctl
# vrsjmp.ll - vrsjmp commandbar
#

(defn is_personal? ()
  (eq? (get (decode :lines (get (exec "uname" "-n") :stdout)) 0)
       "shinjuku.local"))

(defn open_xcode ()
  "Open the active Xcode selected by xcode-select"
  (exec "bash" "-seuo" "pipefail"
        :stdin """
        xcode_app="$(xcode-select -p | grep -oE 'Xcode[^/]+')"
        open -a "$xcode_app"
        """))

(defn toggle_desktop ()
  "Toggle Finder desktop icon visibility"
  (exec "bash" "-seuo" "pipefail"
        :stdin """
        create_desktop="$(defaults read com.apple.finder CreateDesktop 2>/dev/null || echo false)"

        if [ "$create_desktop" = "true" ]; then
            defaults write com.apple.finder CreateDesktop false
        else
            defaults write com.apple.finder CreateDesktop true
        fi

        killall Finder
        """))

(defn toggle_dock_autohide ()
  "Toggle automatic Dock hiding"
  (exec "bash" "-seuo" "pipefail"
        :stdin """
        current="$(defaults read com.apple.Dock autohide)"

        if [ "$current" -eq 1 ]; then
            defaults write com.apple.Dock autohide -bool false
        else
            defaults write com.apple.Dock autohide -bool true
        fi

        killall Dock
        """))

# TODO: Move to scripts/init.ll w/ supervision tree
(bind_srv :system_appearance)
(bind_srv :os_browser)
(bind_srv :os_notify)
(bind_srv :rlist)
(bind_srv :nl_shell)
(bind_srv :nl_scheduler)
(bind_srv :os_screencap)
(bind_srv :things)
(bind_srv :os_apps)
(bind_srv :os_display)
(bind_srv :os_window)
(bind_srv :os_maps)
(bind_srv :os_notes)
(bind_srv :antinote)
(bind_srv :obsidian)
(bind_srv :eden)
(bind_srv :youtube)
(bind_srv :cmd_macro)
(bind_srv :safari_history)
(bind_srv :github)
(bind_srv :os_clipboard)
(try (bind_srv :os_context))
(bind_srv :stickies)

(defn get_items (callback args query)
  "Render a named page using fixed argument values followed by the input text"
  (apply (eval callback) (push args query)))

(defn push_page (callback prompt)
  "Return an instruction to push a lazily rendered page in the GUI"
  (list :push_page :get_items callback :prompt prompt))

# TODO: Revisit the begin_interaction hook: separate immediate page restoration
# from slower context enrichment.
(defn begin_interaction ()
  "Capture context once and return the root page; ordinary on_click protocol"
  (set things_cache nil)
  (def context (try (get_context)))
  (+ (push_page 'root_items "Search commands…")
     '(:title "Home")
     (list :args (list (if (list? context) context '())))))

(defn root_items (context query)
  "Retrieve the root command palette's final ordered items"
  (if (if (eq? query "") false (eq? (get (split "-" query) 0) ""))
    (task_items context query)
    (command_items context query)))

(defn command_items (context query)
  (def candidates (+ (favorite_items)
     (notes_items query)
     (stickies_items query)
     (display_items query)
     (scheduler_items query)
     (eden_items query)
     (rlist_items query)
     (youtube_items query)
     (github_items query)
     (macro_items query)
     (list (make_item "Read Later" '(read_later_page))
           (make_item "Browser History" '(browser_history_page))
           (make_item "Windows" '(call_interactively 'focus_window)))
     (interactive_items context)))
  # Rank all fields together: a weak title match must not outrank an app name.
  (+ (fuzzy_match query candidates (fn (item)
       (list (get item :title)
             (if (get item :subtitle) (get item :subtitle) "")
             (if (get item :aside) (get item :aside) "")
             (display item))))
     (query_items query)))

(defn make_item (title command)
  "Create an item with TITLE and COMMAND"
  (list :title title :on_click command))

(defn make_item_ex (title command hints)
  "Create an item with TITLE and COMMAND and HINTS"
  (list :hints hints :title title :on_click command))

(defn interactive_commands ()
  (filter (ls_env) (fn (name) (eq? (get (meta (eval name)) :interactive) true))))

(defn command_title (name)
  (def metadata (meta (eval name)))
  (if (get metadata :doc) (get metadata :doc) (display name)))

(defn interactive_items (context)
  # One row per command, not one row per captured object. Window commands live
  # on the Windows page; its secondary actions can fill additional arguments.
  (map (filter (interactive_commands) (fn (name)
    (if (contains? '(focus_window move_window open_antinote_note) name) false
      (let ((args (get (meta (eval name)) :args)))
        (if (empty? args) true
          (if (not? (empty? (filter context (fn (entity) (accepts_context? name entity))))) true
            (not? (empty? (get_entity_completions (get (get args 0) :type))))))))))
    (fn (name)
      (def matches (filter context (fn (entity) (accepts_context? name entity))))
      (def choose (list 'call_interactively (list 'quote name)))
      (+ (make_item (if (eq? name 'save_page)
                       (if (empty? matches) "Save a Page to Read Later…" (command_title name))
                       (command_title name))
           (if (empty? matches) choose
             (list 'continue_call (list 'quote name) (list 'quote (list (get matches 0))))))
         (list :actions
           (+ (map matches (fn (entity)
                (make_item (entity_title entity)
                  (list 'continue_call (list 'quote name) (list 'quote (list entity))))))
              (let ((args (get (meta (eval name)) :args)))
                (if (empty? args) '()
                  (if (empty? (get_entity_completions (get (get args 0) :type))) '()
                    (list (make_item "Choose…" choose)))))))))))

(defn accepts_context? (name entity)
  (def args (get (meta (eval name)) :args))
  (if (empty? args) false
    (eq? (get (get args 0) :type) (get entity 0))))

(defn save_page (page)
  "Save to Read Later"
  (interactive :web/page)
  (def result (feedbin_call (list :feedbin_save (get page :url) (get page :title))))
  (if (empty? result) (error "Feedbin did not save the page. Check its connection and authentication.") result))

(defn copy_page_url (page)
  "Copy Page URL"
  (interactive :web/page)
  (set_clipboard (get page :url)))

(defn copy_selected_text (selection)
  "Copy Selected Text"
  (interactive :text)
  (set_clipboard (get selection :value)))

(defn call_interactively (name)
  "Fill a named command's required arguments using completion pages, then call it"
  (continue_call name '()))

(defn continue_call (name values)
  (def signature (get (meta (eval name)) :args))
  (if (eq? (len values) (len signature))
    (apply (eval name) values)
    (let ((arg (get signature (len values))))
      (if (eq? (get arg :type) nil)
        (error (format "No entity type for {}" (display (get arg :name)))))
      (list :push_page :get_items 'call_items
            :args (list name values)
            :title (command_title name)
            :prompt (format "{} · {}" (command_title name) (display (get arg :name)))))))

(defn entity_title (entity)
  (if (get entity :title)
    (if (get entity :app) (format "{} — {}" (get entity :app) (get entity :title))
      (get entity :title))
    (display entity)))

(defn call_items (name values query)
  (def arg (get (get (meta (eval name)) :args) (len values)))
  (def type (get arg :type))
  (def providers (get_entity_completions type))
  (if (empty? providers) (error (format "No completions configured for {}" (display type))))
  (def entities '())
  (map providers (fn (provider)
    (def found (try (apply (eval provider) '())))
    (if (list? found)
      (map found (fn (entity)
        (if (list? entity)
          (if (eq? (get entity 0) type)
            (if (not? (contains? entities entity))
              (set entities (push entities entity))))))))))
  (map (fuzzy_match query entities display) (fn (entity)
    (+ (make_item (if (get entity :title) (get entity :title) (entity_title entity))
         (list 'continue_call (list 'quote name) (list 'quote (push values entity))))
       (list :subtitle (if (eq? type :os/app)
                         (get entity :bundle_id)
                         (get entity :app))
             :aside (if (get entity :id) (str (get entity :id)) nil)
             :actions (+ (entity_actions entity)
                         (if (eq? type :os/window) (window_actions entity) '())))))))

(defn entity_actions (entity)
  "Secondary actions come from the commands imported into this service"
  (map (filter (interactive_commands) (fn (name) (accepts_context? name entity)))
    (fn (name)
      (make_item (command_title name)
        (list 'continue_call (list 'quote name) (list 'quote (list entity)))))))

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
       (make_item "Open App"
                  (list 'open_app query))
       (make_item "Open URL"
                  (list 'open_url query))
       (make_item "Do It"
                  (list 'codegen_exec query))
       (make_item "Search Amazon"
                  (list 'open_url (format "https://www.amazon.com/s?k={}" query)))
       )))

(defn display_items (query)
  "Return item for display commands"
  (if (not? (contains? query "d:"))
    '()
    (map (list_alternative_resolutions) (fn (r) (make_item (format "d: {}" r) (list 'select_resolution r))))))

(defn window_actions (window)
  "Apply existing layout commands to the chosen window, not the launcher"
  (map '(("Split" window_split) ("Fullscreen" window_fullscreen)
         ("Center" window_center) ("Left Half" window_left) ("Right Half" window_right)
         ("Top Left" window_top_left) ("Top Right" window_top_right)
         ("Bottom Left" window_bottom_left) ("Bottom Right" window_bottom_right))
    (fn (action)
      (make_item (get action 0)
        (list 'begin (list 'focus_window (list 'quote window)) (list (get action 1)))))))

(defn scheduler_items (query)
  "Return item for scheduler commands"
  # Only match if query contains win
  (if (not? (contains? query "schedule"))
        '()
      (list
       (make_item "Schedule - Tomorrow" '(schedule_the_day "tomorrow"))
       (make_item "Schedule - Today" '(schedule_the_day "today")))))


(defn trim_text (text)
  # Keep internal whitespace intact; trim only the edges without a subprocess.
  (def result "")
  (def pending "")
  (map (split "" text) (fn (char)
    (if (contains? '("" " " "\t" "\r" "\n") char)
      (if (not? (eq? result "")) (set pending (str pending char)))
      (begin (set result (str result pending char)) (set pending "")))))
  result)

(def things_cache nil)

(defn task_items (context query)
  (def index 0)
  (def title (trim_text (apply join (+ '("-")
    (filter (split "-" query) (fn (part)
      (set index (+ index 1)) (not? (eq? index 1))))))))
  (def create (if (eq? title "") '(error "Task title is empty")
                 (list 'things_add title "")))
  (+ (list (+ (make_item "Add to Things Inbox" create)
              (list :subtitle (if (eq? title "") nil title))))
     (safari_task_items context)
     (matching_task_items title)))

(defn matching_task_items (query)
  (if (eq? query "") '()
    (begin
      # Read Things once per interaction; each subsequent keystroke stays local.
      (if (eq? things_cache nil) (set things_cache (get_things_tasks)))
      (map (fuzzy_match query things_cache (fn (task)
               (list (get task :title) (get task :notes) (display task))))
        (fn (task)
          (def excerpt (match_excerpt query (get task :notes)))
          (+ (make_item (get task :title) (list 'open_things_task (list 'quote task)))
             (list :subtitle (if excerpt excerpt (get task :notes)) :aside "Things")))))))

(defn safari_task_items (context)
  # Use the captured origin, not whichever app is focused after opening jmp.
  (def windows (filter context (fn (entity) (eq? (get entity 0) :os/window))))
  (if (empty? windows) '()
    (if (not? (eq? (get (get windows 0) :app) "Safari")) '()
      (let ((pages (filter context (fn (entity)
                     (if (eq? (get entity 0) :web/page)
                       (if (get entity :url) (not? (eq? (get entity :url) "")) false)
                       false)))))
        (if (empty? pages) '()
          (let ((page (get pages 0)))
            (def url (get page :url))
            (def title (if (get page :title) (trim_text (get page :title)) ""))
            (if (eq? title "") (set title url))
            (list (+ (make_item "Add Safari Tab to Things" (list 'things_add title url))
                     (list :subtitle title)))))))))

(defn feedbin_call (message)
  "Let transport errors reach the palette toast rather than masquerading as no results"
  (call (find_srv :feedbin) message))

(defn read_later_page ()
  (+ (push_page 'read_later_items "Search saved pages…") '(:title "Read Later" :debounce_ms 200)))

(def pages_collection_cache nil)
(defn pages_collection ()
  "Resolve the indexed Pages feed; never silently fall back to all feeds"
  (if (not? pages_collection_cache)
    (let ((pages (filter (feedbin_call '(:feedbin_collections))
                  (fn (collection)
                    (if (eq? (get collection :kind) "feed")
                      (if (eq? (get collection :name) "Pages") true
                        (if (get collection :feed_url)
                          (contains? (get collection :feed_url) "pages.feedbinusercontent.com/")
                          false))
                      false)))))
      (if (not? (empty? pages))
        (set pages_collection_cache (str "feed:" (get (get pages 0) :id))))))
  pages_collection_cache)

(defn read_later_items (query)
  "List recent saved pages or search the indexed Pages collection"
  (def entries
    (if (eq? query "")
      (feedbin_call '(:feedbin_saved_pages 20))
      (let ((collection (pages_collection)))
        (if collection
          (feedbin_call (list :feedbin_search_in collection query 20))
          '()))))
  (map (filter entries (fn (entry) (if (list? entry) (get entry :url) false))) (fn (entry)
    (def url (get entry :url))
    (def title (if (get entry :title) (get entry :title) url))
    (def host (get (split "/" url) 2))
    (def saved (get entry :created_at))
    (+ (make_item title (list 'open_url url))
       (list :subtitle (str (if host host url)
                           (if saved (str " · Saved " (get (split "T" saved) 0)) ""))
             :actions (list
               (make_item "Open in Browser" (list 'open_url url))
               (make_item "Copy URL" (list 'set_clipboard url))
               (make_item "Copy Title and URL" (list 'set_clipboard (str title "\n" url)))))))))

(defn notes_items (query)
  "(notes_items) - Returns markup for notes"
  (if (not? (contains? query "n:"))
    '()
    (map (get_notes) (fn (n) (list :title (format "n: {}" (get n :title))
                                   :on_click (list 'open_note (get n :id)))))))

(def stickies_get_cache '())
(defn stickies_items (query)
  "(stickies_items) - Returns markup for Stickies"
  (if (not? (contains? query "s:"))
    '()
      (begin
       (if (eq? query "s:") (set stickies_get_cache (stickies_get)))
       (map stickies_get_cache (fn (n) (list :title (format "s: {}" (get n :title))
                                               :on_click (list 'stickies_open (get n :title))))))))

(def obsidian_cache '())

(defn obsidian_page ()
  (set obsidian_cache (get_obsidian_files))
  (+ (push_page 'obsidian_items "Search Obsidian files…") '(:title "Obsidian")))

(defn obsidian_items (query)
  (map (fuzzy_match query obsidian_cache display) (fn (note)
    (+ (make_item (get note :title) (list 'open_obsidian_file (get note :file)))
       (list :subtitle (get note :file))))))

(defn youtube_items (query)
  "(youtube_items QUERY) - Returns markup for youtube items"
  (if (not? (contains? query "yt:"))
    (list
     (make_item "Download YT Video" '(download_video_active_tab)))
    (map (list_videos) (fn (n) (list :title (format "yt: {}" (get n :title))
                                     :on_click (list 'open_file (get n :path)))))))

(defn eden_items (query)
  "(eden_items QUERY) - Returns markup for eden tabs"
  (if (not? (contains? query "eden:"))
    '()
      (+
       (list (make_item "eden: Ask AI" (list 'spawn (list 'fn '() (list 'eden_ai query)))))
       (map (eden_list) (fn (e)
           (list :title (format "eden: {}" (get e :title))
                 :on_click (list 'eden_open (get e :id))))))))

(def browser_history_cache '())

(defn browser_history_page ()
  (refresh_safari_history)
  (set browser_history_cache (get_safari_history))
  (+ (push_page 'browser_history_items "Search recent Safari history…")
     '(:title "Browser History")))

(defn browser_history_items (query)
  (map (fuzzy_match query browser_history_cache display) (fn (entry)
    (def url (get entry :url))
    (def page (list :web/page :title (get entry :title) :url url))
    (+ (make_item (get entry :title) (list 'open_url url))
       (list :subtitle (get (split "/" url) 2) :aside (get entry :visited)
             :actions (entity_actions page))))))

(def antinote_cache '())

(defn antinote_page ()
  (set antinote_cache (get_antinote_notes))
  (+ (push_page 'antinote_items "Search Antinote notes…")
     '(:title "Antinote")))

(defn antinote_items (query)
  (map (fuzzy_match query antinote_cache display) (fn (note)
    (def excerpt (match_excerpt query (get note :content)))
    (+ (make_item (get note :title) (list 'open_antinote_note (list 'quote note)))
       (list :subtitle (if excerpt excerpt (get note :modified))
             :aside (if excerpt
                      (if (get note :modified) (get (split " " (get note :modified)) 0) nil)
                      nil)
             :actions (list
               (make_item "Copy Note" (list 'set_clipboard (get note :content)))
               (make_item "Open Antinote" '(open_antinote))))))))

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
         (+ (make_item_ex "Browser" '(open_app "Safari") 'b) '(:aside "Safari"))
         # (make_item_ex "Deta Surf" '(open_app "Surf") 'b)
         (+ (make_item_ex "Terminal" '(open_app "Ghostty") 't) '(:aside "Ghostty"))
         (make_item_ex "TextEdit" '(open_app "TextEdit") 'te)
         # (make_item "Terminal" '(open_app "Alacritty"))
         (+ (make_item "Things" '(open_app "Things3")) '(:aside "Things3"))
         (make_item "Screen Sharing" '(open_app "Screen Sharing"))
         (make_item "Telegram" '(open_app "Telegram"))
         (make_item "Messages" '(open_app "Messages"))
         (make_item "YouTube Music" '(open_app "YouTube Music"))
         (make_item "Notes" '(open_app "Notes"))
         (make_item "Reminders" '(open_app "Reminders"))
         (make_item "Shortcuts" '(open_app "Shortcuts"))
         # (make_item "Mail" '(open_app "Spark"))
         # (make_item "Mail" '(open_app "Mimestream"))
         (+ (make_item "Mail" '(open_url "https://mail.google.com")) '(:aside "mail.google.com"))
         (make_item "Cal" '(open_app "Calendar"))
         # (make_item "Cal" '(open_app "Notion Calendar"))
         (make_item "Find My" '(open_app "FindMy"))
         (make_item "Soulver" '(open_app "Soulver 3"))
         (make_item "1Password" '(open_app "1Password"))
         (make_item "TLDraw" '(open_url "https://www.tldraw.com"))
         (make_item "Xcode" '(open_xcode)) # TODO: Built-in regex
         (make_item "Chrome" '(open_app "Google Chrome"))
         (make_item "Obsidian" '(open_app "Obsidian"))
         (make_item "Browse Obsidian" '(obsidian_page))
         (make_item "Script Debugger" '(open_app "Script Debugger"))
         (make_item "ProxyMan" '(open_app "ProxyMan"))
         (make_item_ex "Reeder" '(open_app "Reeder") 'reeder)
         (make_item_ex "Feedbin" '(open_url "https://feedbin.com/") 'feedbin)
         (make_item "Copy Feedbin Email" '(set_clipboard "leo.001@feedb.in"))
         (make_item_ex "Habitica" '(open_url "https://habitica.com/") 'habitica)
         (make_item_ex "Codex" '(open_app "Codex") 'codex)
         (make_item "VirtualBuddy" '(open_app "VirtualBuddy"))
         (make_item "VirtualBuddy - Shared" '(open_file "~/VirtualBuddy-Shared"))
         (make_item "Stickies" '(open_app "Stickies"))
         (make_item "Photos" '(open_app "Photos"))

         (make_item "Distill" '(open_app "Distill"))
         (make_item "Antinote" '(open_antinote))
         (make_item "Browse Antinote" '(antinote_page))
         (make_item "Patina" '(open_app "Patina"))

         (make_item "Marketplace" '(open_url "https://www.facebook.com/marketplace"))

         (make_item "UI Browser" '(open_app "UI Browser"))

         # Assistants
         (make_item "Claude" '(open_app "Claude"))
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
         (make_item "Toggle Desktop" '(toggle_desktop))
         (make_item "Toggle Dock" '(toggle_dock_autohide))
         (make_item "Toggle QuickShade" '(toggle_quick_shade))
         (make_item "Open in Wayback" '(active_tab_open_wayback))
         (make_item "Show Desktop" '(show_desktop))
         (make_item "Toggle DND" '(toggle_do_not_disturb)))

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
  (def result (eval cmd))
  # `exec` returns exit status as data. A failed shell action should be a toast,
  # not a successful close just because evaluating the expression succeeded.
  (if (list? result)
    (if (not? (eq? (get result :exit) nil))
      (if (not? (eq? (get result :exit) 0))
        (error (str "Command failed: " (get result :stderr))))))
  (if (list? result)
    (if (eq? (get result 0) :push_page) result :close)
    :close))

(spawn_srv :vrsjmp :interface '(get_items on_click))
