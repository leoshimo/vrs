#!/usr/bin/env vrsctl
# vrsjmp.ll - vrsjmp commandbar
#

(defn! is_personal? ()
  (eq? (get (decode :lines (get (exec "uname" "-n") :stdout)) 0)
       "shinjuku.local"))

(defn! open_xcode ()
  "Open the active Xcode selected by xcode-select"
  (exec "bash" "-seuo" "pipefail"
        :stdin """
        xcode_app="$(xcode-select -p | grep -oE 'Xcode[^/]+')"
        open -a "$xcode_app"
        """))

(defn! toggle_desktop ()
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

(defn! toggle_dock_autohide ()
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
(bind_srv :codex)
(bind_srv :tailscale)
(bind_srv :obsidian)
(bind_srv :youtube)
(bind_srv :cmd_macro)
(bind_srv :safari_history)
(bind_srv :github)
(bind_srv :os_clipboard)
(try (bind_srv :os_context))
(bind_srv :stickies)

(def pending_inputs '())

(defn! pending_requests ()
  "Drop abandoned requests. The waiting caller consumes these liveness messages."
  (set pending_inputs (filter pending_inputs (fn (request)
    (ok? (try (send (get request :owner)
                    (list (get request :id) :pending nil))))))))

(defn! input_request (id)
  (def requests (filter (pending_requests) (fn (request) (eq? (get request :id) id))))
  (if (empty? requests) (error "This input request has ended"))
  (get requests 0))

(defn! enqueue_input (id owner page)
  "Store a page before publishing a wakeup; the GUI fetches it on opening."
  (if (not? (eq? (get page 0) :push_page)) (error "Input needs a :push_page description"))
  (if (not? (symbol? (get page :get_items))) (error "Input needs a page callback"))
  (def args (get page :args))
  (if (eq? args nil) (set args '()))
  (if (not? (list? args)) (error "Page arguments must be a list"))
  (def prompt (str (or! (get page :prompt) "Choose…")))
  (def title (str (or! (get page :title) prompt)))
  (def prepared (list :push_page :get_items (get page :get_items)
                     :title title :prompt prompt
                     :args (concat (list id) args)
                     :on_cancel `(cancel_input ,id)))
  (pending_requests)
  (set pending_inputs (push pending_inputs (list :id id :owner owner :page prepared)))
  (show_gui)
  :ok)

(defn! finish_input (id value)
  "Return a value to the waiting evaluation; the selected form is not evaluated."
  (def request (input_request id))
  (set pending_inputs (filter pending_inputs (fn (request) (not? (eq? (get request :id) id)))))
  (send (get request :owner) (list id :ok value))
  :close)

(defn! cancel_input (id)
  (def requests (filter pending_inputs (fn (request) (eq? (get request :id) id))))
  (set pending_inputs (filter pending_inputs (fn (request) (not? (eq? (get request :id) id)))))
  (map requests (fn (request) (try (send (get request :owner) (list id :cancel nil)))))
  :close)

(defn! resume_input (id)
  (get (input_request id) :page))

(defn! pending_input_items (query)
  (fuzzy_match query (map (pending_requests) (fn (request)
    (list :title (get (get request :page) :title) :subtitle "Waiting for input"
          :on_click `(resume_input ,(get request :id))
          :actions (list (make_item "Cancel request" `(cancel_input ,(get request :id))))))) display))

(defn! get_items (callback args query)
  "Render a named page using fixed argument values followed by the input text"
  (apply (eval callback) (push args query)))

(defn! push_page (callback prompt)
  "Return an instruction to push a lazily rendered page in the GUI"
  (list :push_page :get_items callback :prompt prompt))

# TODO: Revisit the begin_interaction hook: separate immediate page restoration
# from slower context enrichment.
(defn! begin_interaction ()
  "Return pending input, or capture context and return Home; ordinary on_click protocol"
  (set things_cache nil)
  (set codex_cache nil)
  (def requests (pending_requests))
  (if (not? (empty? requests)) (get (get requests 0) :page)
    (let ((context (try (get_context))))
      (+ (push_page 'root_items "Search commands…")
         '(:title "Home")
         (list :args (list (if (list? context) context '())))))))

(defn! root_items (context query)
  "Retrieve the root command palette's final ordered items"
  (+ (pending_input_items query)
     (if (and! (not? (eq? query "")) (eq? (get (split "-" query) 0) ""))
       (task_items context query)
       (command_items context query))))

(defn! command_items (context query)
  (def candidates (+ (favorite_items)
     (scheduler_items query)
     (macro_items query)
     (list (make_item "Read Later" '(read_later_page))
           (make_item "Browser History" '(browser_history_page))
           (make_item "Tailscale" '(tailscale_page))
           (make_item "Windows" '(call_interactively 'focus_window))
           (make_item_ex "Configure Display Resolution" '(display_page) 'd)
           (make_item_ex "Browse GitHub PRs" '(github_page) 'gh)
           (make_item "Download YT Video" '(download_video_active_tab)))
     (interactive_items context)))
  # Rank all fields together: a weak title match must not outrank an app name.
  (+ (fuzzy_match query candidates (fn (item)
       (list (get item :title)
             (if (eq? (get item :hints) nil) "" (display (get item :hints)))
             (if (get item :subtitle) (get item :subtitle) "")
             (if (get item :aside) (get item :aside) "")
             (display item))))
     (query_items query)))

(defn! make_item (title command)
  "Create an item with TITLE and COMMAND"
  (list :title title :on_click command))

(defn! make_item_ex (title command hints)
  "Create an item with TITLE and COMMAND and HINTS"
  (list :hints hints :title title :on_click command))

(defn! interactive_commands ()
  (filter (ls_env) (fn (name) (eq? (get (meta (eval name)) :interactive) true))))

(defn! call_form (name arguments)
  "Fill remaining positions with the function's actual parameter names."
  (concat (list name) arguments
          (map (slice (get (meta (eval name)) :args) (len arguments))
               (fn (arg) (get arg :name)))))

(defn! input_page (id callback args title prompt)
  (input_request id)
  (list :push_page :get_items callback :args (concat (list id) args)
        :title title :prompt prompt :on_cancel `(cancel_input ,id)))

(defn! function_items (id query)
  "Search bound service methods; private helpers are not picker entries."
  (input_request id)
  (def names (filter (ls_env) (fn (name)
    (def value (eval name))
    (and! (lambda? value) (keyword? (get (meta value) :service))))))
  (map (fuzzy_match query names (fn (name)
           (list (display name) (or! (get (meta (eval name)) :doc) ""))))
    (fn (name)
      (def form (call_form name '()))
      (list :title (display form)
            :subtitle (get (meta (eval name)) :doc)
            :on_click `(finish_input ,id ',form)
            :actions (list
              (make_item "Fill arguments" `(fill_call ,id ',name '())))))))

(defn! fill_call (id name arguments)
  "Use interactive completion providers to build source without executing it."
  (def signature (get (meta (eval name)) :args))
  (if (eq? (len arguments) (len signature))
    (finish_input id (call_form name arguments))
    (let ((arg (get signature (len arguments))))
      (if (eq? (get arg :type) nil)
        (error (format "No entity type for {}" (display (get arg :name)))))
      (input_page id 'fill_call_items (list name arguments) (display name)
                  (format "{} · {}" (display name) (display (get arg :name)))))))

(defn! literal_form (value)
  (if (or! (list? value) (symbol? value)) (list 'quote value) value))

(defn! fill_call_value (id name arguments value)
  (fill_call id name (push arguments (literal_form value))))

(defn! fill_call_items (id name arguments query)
  (input_request id)
  (def arg (get (get (meta (eval name)) :args) (len arguments)))
  (def type (get arg :type))
  (if (empty? (get_entity_completions type))
    (error (format "No completions configured for {}" (display type))))
  (map (fuzzy_match query (argument_entities type) display) (fn (entity)
    (+ (make_item (entity_title entity)
         `(fill_call_value ,id ',name ',arguments ',entity))
       (list :subtitle (get entity :app))))))

(defn! command_title (name)
  (def metadata (meta (eval name)))
  (or! (get metadata :doc) (display name)))

(defn! interactive_items (context)
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
      (def choose `(call_interactively ',name))
      (+ (make_item (if (eq? name 'save_page)
                       (if (empty? matches) "Save a Page to Read Later…" (command_title name))
                       (command_title name))
           (if (empty? matches) choose
             `(continue_call ',name '(,(get matches 0)))))
         (list :actions
           (+ (map matches (fn (entity)
                (make_item (entity_title entity)
                  `(continue_call ',name '(,entity)))))
              (let ((args (get (meta (eval name)) :args)))
                (if (empty? args) '()
                  (if (empty? (get_entity_completions (get (get args 0) :type))) '()
                    (list (make_item "Choose…" choose)))))))))))

(defn! accepts_context? (name entity)
  (def args (get (meta (eval name)) :args))
  (if (empty? args) false
    (eq? (get (get args 0) :type) (get entity 0))))

(defn! save_page (page)
  "Save to Read Later"
  (interactive :web/page)
  (def result (feedbin_call (list :feedbin_save (get page :url) (get page :title))))
  (if (empty? result) (error "Feedbin did not save the page. Check its connection and authentication.") result))

(defn! copy_page_url (page)
  "Copy Page URL"
  (interactive :web/page)
  (set_clipboard (get page :url)))

(defn! copy_selected_text (selection)
  "Copy Selected Text"
  (interactive :text)
  (set_clipboard (get selection :value)))

(defn! call_interactively (name)
  "Fill a named command's required arguments using completion pages, then call it"
  (continue_call name '()))

(defn! continue_call (name values)
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

(defn! entity_title (entity)
  (if (get entity :title)
    (if (get entity :app) (format "{} — {}" (get entity :app) (get entity :title))
      (get entity :title))
    (display entity)))

(defn! call_items (name values query)
  (def arg (get (get (meta (eval name)) :args) (len values)))
  (def type (get arg :type))
  (def providers (get_entity_completions type))
  (if (empty? providers) (error (format "No completions configured for {}" (display type))))
  (def entities (argument_entities type))
  (map (fuzzy_match query entities display) (fn (entity)
    (+ (make_item (or! (get entity :title) (entity_title entity))
         `(continue_call ',name ',(push values entity)))
       (list :subtitle (if (eq? type :os/app)
                         (get entity :bundle_id)
                         (get entity :app))
             :aside (if (get entity :id) (str (get entity :id)) nil)
             :actions (+ (entity_actions entity)
                         (if (eq? type :os/window) (window_actions entity) '())))))))

(defn! entity_actions (entity)
  "Secondary actions come from the commands imported into this service"
  (map (filter (interactive_commands) (fn (name) (accepts_context? name entity)))
    (fn (name)
      (make_item (command_title name)
        `(continue_call ',name '(,entity))))))

(defn! argument_entities (type)
  "Shared argument choices for interactive execution and call construction."
  (def providers (if (eq? type nil) '() (get_entity_completions type)))
  (def entities '())
  (map providers (fn (provider)
    (def found (try (apply (eval provider) '())))
    (if (list? found)
      (map found (fn (entity)
        (when! (and! (list? entity)
                    (eq? (get entity 0) type)
                    (not? (contains? entities entity)))
          (set entities (push entities entity))))))))
  entities)

# TODO: Query should be rule-based? I.e. "Search DWIM" - if URL, if App Name, if Bundle ID, if location (?), if long, etc
(defn! query_items (query)
  "Return a dynamic list of item for current query"
  (if (not? query) '()
      (list
       (make_item "Search Google"
                  `(open_url ,(format "http://google.com/search?q={}" query)))
       (make_item "Search Maps"
                  `(open_maps_search ,query))
       (make_item "Search Perplexity"
                  `(open_url ,(format "http://perplexity.ai/?q={}&copilot=true" query)))
       (make_item "Search YT Music"
                  `(open_url ,(format "http://music.youtube.com/search?q={}" query)))
       (make_item "Open App"
                  `(open_app ,query))
       (make_item "Open URL"
                  `(open_url ,query))
       (make_item "Do It"
                  `(codegen_exec ,query))
       (make_item "Search Amazon"
                  `(open_url ,(format "https://www.amazon.com/s?k={}" query)))
       )))

(def resolutions_cache '())
(defn! display_page ()
  (set resolutions_cache (list_alternative_resolutions))
  (+ (push_page 'display_items "Search resolutions…") '(:title "Configure Display Resolution")))

(defn! display_items (query)
  (map (fuzzy_match query resolutions_cache str) (fn (resolution)
    (make_item resolution `(select_resolution ,resolution)))))

(defn! window_actions (window)
  "Apply existing layout commands to the chosen window, not the launcher"
  (map '(("Split" window_split) ("Fullscreen" window_fullscreen)
         ("Center" window_center) ("Left Half" window_left) ("Right Half" window_right)
         ("Top Left" window_top_left) ("Top Right" window_top_right)
         ("Bottom Left" window_bottom_left) ("Bottom Right" window_bottom_right))
    (fn (action)
      (make_item (get action 0)
        `(begin (focus_window ',window) (,(get action 1)))))))

(defn! scheduler_items (query)
  "Return item for scheduler commands"
  (if (not? (contains? query "schedule"))
        '()
      (list
       (make_item "Schedule - Tomorrow" '(schedule_the_day "tomorrow"))
       (make_item "Schedule - Today" '(schedule_the_day "today")))))


(defn! trim_text (text)
  # Keep internal whitespace intact; trim only the edges without a subprocess.
  (def result "")
  (def pending "")
  (map (split "" text) (fn (char)
    (if (contains? '("" " " "\t" "\r" "\n") char)
      (if (not? (eq? result "")) (set pending (str pending char)))
      (begin (set result (str result pending char)) (set pending "")))))
  result)

(def things_cache nil)

(defn! task_items (context query)
  (def index 0)
  (def title (trim_text (apply join (+ '("-")
    (filter (split "-" query) (fn (part)
      (set index (+ index 1)) (not? (eq? index 1))))))))
  (def create (if (eq? title "") '(error "Task title is empty")
                 `(things_add ,title "")))
  (+ (list (+ (make_item "Add to Things Inbox" create)
              (list :subtitle (if (eq? title "") nil title))))
     (safari_task_items context)
     (matching_task_items title)))

(defn! matching_task_items (query)
  (if (eq? query "") '()
    (begin
      # Read Things once per interaction; each subsequent keystroke stays local.
      (if (eq? things_cache nil) (set things_cache (get_things_tasks)))
      (map (fuzzy_match query things_cache (fn (task)
               (list (get task :title) (get task :notes) (display task))))
        (fn (task)
          (def excerpt (match_excerpt query (get task :notes)))
          (+ (make_item (get task :title) `(open_things_task ',task))
             (list :subtitle (if excerpt excerpt (get task :notes)) :aside "Things")))))))

(defn! safari_task_items (context)
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
            (list (+ (make_item "Add Safari Tab to Things" `(things_add ,title ,url))
                     (list :subtitle title)))))))))

(defn! feedbin_call (message)
  "Let transport errors reach the palette toast rather than masquerading as no results"
  (call (find_srv :feedbin) message))

(defn! read_later_page ()
  (+ (push_page 'read_later_items "Search saved pages…") '(:title "Read Later" :debounce_ms 200)))

(def pages_collection_cache nil)
(defn! pages_collection ()
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

(defn! read_later_items (query)
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
    (+ (make_item title `(open_url ,url))
       (list :subtitle (str (if host host url)
                           (if saved (str " · Saved " (get (split "T" saved) 0)) ""))
             :actions (list
               (make_item "Open in Browser" `(open_url ,url))
               (make_item "Copy URL" `(set_clipboard ,url))
               (make_item "Copy Title and URL" `(set_clipboard ,(str title "\n" url)))))))))

(def notes_cache nil)
(defn! apple_notes_page ()
  (set notes_cache (get_notes))
  (+ (push_page 'apple_notes_items "Search Apple Notes…") '(:title "Apple Notes")))

(defn! apple_notes_items (query)
  (map (fuzzy_match query notes_cache display) (fn (note)
    (make_item (get note :title) `(open_note ,(get note :id))))))

(def stickies_get_cache '())
(defn! stickies_page ()
  (set stickies_get_cache (stickies_get))
  (+ (push_page 'stickies_items "Search Stickies…") '(:title "Stickies")))

(defn! stickies_items (query)
  (map (fuzzy_match query stickies_get_cache display) (fn (note)
    (make_item (get note :title) `(stickies_open ,(get note :title))))))

(def codex_cache nil)
(defn! codex_page ()
  (set codex_cache nil)
  (+ (push_page 'codex_items "Search Codex threads…") '(:title "Codex Threads")))

(defn! codex_items (query)
  # One snapshot per page visit; keystrokes search metadata in memory.
  (if (eq? codex_cache nil) (set codex_cache (get_codex_threads)))
  (map (fuzzy_match query codex_cache (fn (thread)
           (list (get thread :title) (get thread :cwd) (get thread :host)
                 (get thread :id) (get thread :url)
                 (if (get thread :modified) (get thread :modified) "")
                 (if (get thread :unread) "Unread" ""))))
    (fn (thread)
      (def project (get (split "/" (get thread :cwd)) -1))
      (def host (get (split ":" (get thread :host)) -1))
      (+ (make_item (get thread :title) `(open_codex_thread ',thread))
         (list :subtitle (str (if (eq? project "") "" (str project " · ")) host)
               :aside (str (if (get thread :unread) "Unread" "")
                           (if (get thread :modified)
                             (str (if (get thread :unread) " · " "") (get thread :modified)) "")))))))

(def tailscale_cache nil)
(defn! tailscale_page ()
  (set tailscale_cache (get_tailscale_snapshot))
  (+ (push_page 'tailscale_items "Search devices and web endpoints…")
     (list :title (str "Tailscale · " (get tailscale_cache :summary)))))

(defn! tailscale_ping (device)
  (def result (ping_tailscale_device device))
  # Pass data as argv, not interpolated AppleScript source.
  (exec "osascript" "-e" """
    on run argv
      display notification (item 2 of argv) with title (item 1 of argv)
    end run
    """ (str "Tailscale · " (get device :title)) result))

(defn! tailscale_device_item (device)
  (def address (or! (get device :ipv4) (get device :ipv6) (get device :dns)))
  (def copies (filter '((:ipv4 "Copy IPv4 Address") (:dns "Copy DNS Name")
                        (:ipv6 "Copy IPv6 Address"))
                     (fn (field) (not? (eq? (get device (get field 0)) nil)))))
  (+ (make_item (get device :title)
       (if address `(set_clipboard ,address) '(error "This device has no address")))
     (list :subtitle (str (or! (get device :dns) (get device :hostname))
                         (if address (str " · " address) ""))
           :aside (str (get device :os) " · "
                       (if (get device :online) "Online" "Offline")
                       (if (get device :self) " · This device" ""))
           :actions (+ (map copies (fn (field)
                         (make_item (get field 1) `(set_clipboard ,(get device (get field 0))))))
                       (if (or! (get device :self)
                                (not? (or! (get device :ipv4) (get device :ipv6)))) '()
                         (list (make_item "Ping Device" `(tailscale_ping ',device))))))))

(defn! tailscale_items (query)
  # One snapshot per page visit; network I/O never runs on each keystroke.
  (if (eq? tailscale_cache nil) (set tailscale_cache (get_tailscale_snapshot)))
  (+ (map (fuzzy_match query (get tailscale_cache :pages) (fn (page)
            (list (get page :title) (get page :device) (get page :url)
                  (get page :description)))) (fn (page)
       (+ (make_item (get page :title) `(open_url ,(get page :url)))
          (list :subtitle (str (get page :device) " · " (get page :url))
                :aside "Web"
                :actions (entity_actions (+ '(:web/page) page))))))
     (map (fuzzy_match query (get tailscale_cache :devices) (fn (device)
            (list (get device :title) (get device :hostname) (get device :os)
                  (or! (get device :dns) "") (or! (get device :ipv4) "")
                  (or! (get device :ipv6) "")
                  (if (get device :online) "Online" "Offline")))) tailscale_device_item)))

(def obsidian_cache '())
(defn! obsidian_page ()
  (set obsidian_cache (get_obsidian_files))
  (+ (push_page 'obsidian_items "Search Obsidian files…") '(:title "Obsidian")))

(defn! obsidian_items (query)
  (map (fuzzy_match query obsidian_cache display) (fn (note)
    (+ (make_item (get note :title) `(open_obsidian_file ,(get note :file)))
       (list :subtitle (get note :file))))))

(def browser_history_cache '())
(defn! browser_history_page ()
  (refresh_safari_history)
  (set browser_history_cache (get_safari_history))
  (+ (push_page 'browser_history_items "Search recent Safari history…")
     '(:title "Browser History")))

(defn! browser_history_items (query)
  (map (fuzzy_match query browser_history_cache display) (fn (entry)
    (def url (get entry :url))
    (def page (list :web/page :title (get entry :title) :url url))
    (+ (make_item (get entry :title) `(open_url ,url))
       (list :subtitle (get (split "/" url) 2) :aside (get entry :visited)
             :actions (entity_actions page))))))

(def antinote_cache '())
(defn! antinote_page ()
  (set antinote_cache (get_antinote_notes))
  (+ (push_page 'antinote_items "Search Antinote notes…")
     '(:title "Antinote")))

(defn! antinote_items (query)
  (map (fuzzy_match query antinote_cache display) (fn (note)
    (def excerpt (match_excerpt query (get note :content)))
    (+ (make_item (get note :title) `(open_antinote_note ',note))
       (list :subtitle (if excerpt excerpt (get note :modified))
             :aside (if excerpt
                      (if (get note :modified) (get (split " " (get note :modified)) 0) nil)
                      nil)
             :actions (list
               (make_item "Copy Note" `(set_clipboard ,(get note :content)))
               (make_item "Open Antinote" '(open_antinote))))))))

(def github_cache '())
(defn! github_page ()
  (refresh_pull_requests)
  (set github_cache (get_pull_requests))
  (+ (push_page 'github_items "Search pull requests…") '(:title "GitHub PRs")))

(defn! github_items (query)
  (map (fuzzy_match query github_cache display) (fn (pr)
    (+ (make_item (get pr :title) `(open_url ,(get pr :url)))
       (list :subtitle (get pr :url))))))

# TODO: Nice to have "prefix-drop" for these prefixed names
(defn! macro_items (query)
  "(macro_items QUERY) - Returns markup for macro items"
  (if (not? (contains? query "macro:"))
    '()
    (+
     (map (get_macros) (fn (m) (list :title (get m :name)
                                     :on_click `(eval ,(get m :cmds)))))
     (list
      (if (macro_is_recording)
        (make_item "macro: Stop Recording" '(end_macro_record))
        (make_item (format "macro: Start Recording - {}" query) `(start_macro_record ,query)))
      (make_item "macro: Clear Macros" '(clear_macros))
      ))))

(defn! favorite_items ()
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
         (make_item "Browse Apple Notes" '(apple_notes_page))
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
         (make_item "Codex Threads" '(codex_page))
         (make_item "VirtualBuddy" '(open_app "VirtualBuddy"))
         (make_item "VirtualBuddy - Shared" '(open_file "~/VirtualBuddy-Shared"))
         (make_item "Stickies" '(open_app "Stickies"))
         (make_item "Browse Stickies" '(stickies_page))
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

   # Demo-only jump list; not part of the everyday palette.
   # (list (make_item "Add to Jump List" '(add_jump_list_active_tab)))

   # recording
   (list (make_item "Screen Capture" '(start_screencap)))
   ))

(defn! local_items ()
  "Read set of local items if any"
  (def res (try (fread "~/vrsjmp_local.ll")))
  (if (ok? res) res '()))

(defn! on_click (item)
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

(spawn_srv! :vrsjmp :interface '(get_items on_click enqueue_input))
