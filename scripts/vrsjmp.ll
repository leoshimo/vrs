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
(bind_srv :os_keyboard)
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
(bind_srv :stickies)

(def pending_inputs '())
(def action_runs '())

(def ui_config_path "~/.vrsjmp-ui.ll")
(def ui_config '(:theme :neutral :appearance :system))

(defn! validate_ui_config (config)
  (if (not? (list? config)) (error "UI configuration must be a property list"))
  (if (not? (contains? '(:neutral :warm :cool) (get config :theme)))
    (error "Theme must be :neutral, :warm, or :cool"))
  (if (not? (contains? '(:system :light :dark) (get config :appearance)))
    (error "Appearance must be :system, :light, or :dark"))
  config)

(defn! get_ui_config ()
  "Read the palette's persisted theme and appearance."
  (def saved (try (fread ui_config_path)))
  (if (ok? saved) (set ui_config (validate_ui_config saved)))
  ui_config)

(defn! set_ui_config (options)
  "Set :theme (:neutral/:warm/:cool) or :appearance (:system/:light/:dark)."
  (if (not? (list? options)) (error "UI configuration must be a property list"))
  (def current (get_ui_config))
  (def theme (get options :theme))
  (def appearance (get options :appearance))
  (def next (validate_ui_config
    `(:theme ,(if (eq? theme nil) (get current :theme) theme)
      :appearance ,(if (eq? appearance nil) (get current :appearance) appearance))))
  (fdump ui_config_path next)
  (set ui_config next)
  (publish :vrsjmp :config_changed)
  next)

(defn! appearance_page ()
  (+ (push_page 'appearance_items "Search appearance…") '(:title "Palette Appearance")))

(defn! appearance_items (query)
  (def config (get_ui_config))
  (def choices '(("Neutral" :theme :neutral) ("Warm" :theme :warm) ("Cool" :theme :cool)
                ("System" :appearance :system) ("Light" :appearance :light) ("Dark" :appearance :dark)))
  (fuzzy_match query (map choices (fn (choice)
    (def key (get choice 1))
    (def value (get choice 2))
    `(:title ,(get choice 0)
      :aside ,(if (eq? (get config key) value) "Selected" "")
      :on_click (begin (set_ui_config (list ,key ,value)) :refresh))))))

(defn! action_record (id)
  (get (filter action_runs (fn (record) (eq? (get record :id) id))) 0))

(defn! finish_action (id result)
  "Receive an action's result without coupling its lifetime to the GUI window."
  (def record (action_record id))
  (set action_runs (filter action_runs (fn (record) (not? (eq? (get record :id) id)))))
  (if (and! record (err? result))
    (set action_runs (push action_runs
      (+ record (list :error (display result))))))
  (if (and! record (not? (eq? (get record :receiver) nil)))
    (try (send (get record :receiver)
      (list :vrsjmp_action id (if (err? result) (list :error (display result)) :ok)))))
  :ok)

(defn! execute_action (record)
  "Run a captured call in a child; only the palette service updates the records."
  (def owner (self))
  (def id (get record :id))
  (def form (get record :form))
  (def started (try (spawn (fn ()
    (def result (try (begin
      (def value (eval form))
      # Match vrs/execute_command's handling of subprocess results.
      (if (list? value)
        (if (not? (eq? (get value :exit) nil))
          (if (not? (eq? (get value :exit) 0))
            (error (str "Command failed: " (get value :stderr))))))
      value)))
    (call owner (list :finish_action id result))))))
  (if (err? started) (finish_action id started))
  :close)

(defn! start_action (title form)
  "Retain already captured source values and start one user-selected action."
  (def record (list :id (display (ref)) :title title :form form))
  (set action_runs (push action_runs record))
  (execute_action record))

(defn! retry_action (id)
  (def record (action_record id))
  # A second activation while running must not create another attempt.
  (if (and! record (get record :error))
    (begin
      (set record (list :id id :title (get record :title) :form (get record :form)))
      (set action_runs (map action_runs (fn (old)
        (if (eq? (get old :id) id) record old))))
      (execute_action record)))
  :refresh)

(defn! dismiss_action (id)
  (set action_runs (filter action_runs (fn (record) (not? (eq? (get record :id) id)))))
  :refresh)

(defn! failed_action_items (query)
  (fuzzy_match query
    (map (filter action_runs (fn (record) (get record :error))) (fn (record)
      (def id (get record :id))
      `(:title ,(get record :title) :aside "Failed"
        :subtitle ,(str (get record :error) "\n" (display (get record :form)))
        :on_click (retry_action ,id)
        :actions ,(list (make_item "Retry" `(retry_action ,id))
                        (make_item "Dismiss" `(dismiss_action ,id))))))
    (fn (item) (list (get item :title) (get item :subtitle)))))

(defn! direct_action? (form)
  "Recognize ordinary command calls; palette navigation and macro controls stay local."
  (if (and! (list? form) (symbol? (get form 0)))
    (let ((callable (try (eval (get form 0)))))
      (if (lambda? callable)
        (let ((metadata (meta callable)))
          (and! (not? (contains? '(:vrsjmp :cmd_macro) (get metadata :service)))
                (or! (get metadata :interactive) (keyword? (get metadata :service)))))
        false))
    false))

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
  (def prepared `(:push_page :get_items ,(get page :get_items)
                 :title ,title :prompt ,prompt
                 :args ,(concat (list id) args)
                 :on_cancel (cancel_input ,id)))
  (pending_requests)
  (set pending_inputs (push pending_inputs `(:id ,id :owner ,owner :page ,prepared)))
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
    `(:title ,(get (get request :page) :title) :subtitle "Waiting for input"
      :on_click (resume_input ,(get request :id))
      :actions ,(list (make_item "Cancel request" `(cancel_input ,(get request :id)))))))))

(defn! get_items (callback args query)
  "Render a named page using fixed argument values followed by the input text"
  (apply (eval callback) (push args query)))

(defn! push_page (callback prompt)
  "Return an instruction to push a lazily rendered page in the GUI"
  `(:push_page :get_items ,callback :prompt ,prompt))

(defn! root_page ()
  "Return pending input or Home without querying other apps"
  (set things_cache nil)
  (set codex_cache nil)
  (def requests (pending_requests))
  (if (not? (empty? requests)) (get (get requests 0) :page)
    (+ (push_page 'root_items "Search…") '(:title "Home"))))

(defn! root_items (query)
  "Retrieve the root command palette's final ordered items"
  (+ (pending_input_items query)
     (failed_action_items query)
     (if (and! (not? (eq? query "")) (eq? (get (split "-" query) 0) ""))
       (task_items query)
       (command_items query))))

(defn! command_items (query)
  (def candidates (+ (favorite_items)
     (scheduler_items query)
     (macro_items query)
     (list (make_item "Read Later" '(read_later_page))
           (make_item "Browse Functions" '(browse_functions_page))
           (make_item "Browse Services" '(browse_services_page))
           (make_item "Browser History" '(browser_history_page))
           (make_item "iCloud Tabs" '(cloud_tabs_page))
           (make_item "Tailscale" '(tailscale_page))
           (make_item "Windows" '(call_interactively 'focus_window))
           (make_item_ex "Display Resolution" '(display_page) 'd)
           (make_item "Toggle Keyboard Backlight" '(toggle_keyboard_backlight))
           (make_item_ex "Browse GitHub PRs" '(github_page) 'gh)
           (make_item "Download YT Video" '(download_video_active_tab)))
     (interactive_items)))
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
  `(:title ,title :on_click ,command))

(defn! make_item_ex (title command hints)
  "Create an item with TITLE and COMMAND and HINTS"
  `(:hints ,hints :title ,title :on_click ,command))

(defn! input_page (id callback args title prompt)
  (input_request id)
  `(:push_page :get_items ,callback :args ,(concat (list id) args)
    :title ,title :prompt ,prompt :on_cancel (cancel_input ,id)))

(defn! choose_items (id choices mode query)
  (input_request id)
  (def fields (eq? mode :fields))
  (def rows (map choices (fn (choice)
    (def value (if fields (get choice 1) choice))
    (def title (if fields (display (get choice 0)) (choice_label value)))
    (def detail (if (list? value) (display value) (choice_label value)))
    `(:title ,title
      :subtitle ,(if (eq? title detail) "" detail)
      :on_click (finish_input ,id ',value)))))
  (fuzzy_match query rows (fn (row)
    (list (get row :title) (get row :subtitle)))))

(defn! function_item (name)
  (def metadata (meta (eval name)))
  `(:title ,(display (call_form name '()))
    :subtitle ,(get metadata :doc)
    :aside ,(display (get metadata :service))))

(defn! browse_functions_page ()
  (+ (push_page 'service_function_items "Search functions or services…")
     '(:title "Browse Functions")))

(defn! service_function_items (query)
  (map (service_functions query) (fn (name)
    (+ (function_item name)
       `(:on_click (call_interactively ',name))))))

(defn! browse_services_page ()
  (+ (push_page 'service_items "Search services…") '(:title "Browse Services")))

(defn! service_items (query)
  (map (service_names query) (fn (service)
    (make_item (display service) `(browse_service_page ,service)))))

(defn! browse_service_page (service)
  (+ (push_page 'interface_function_items "Search interface functions…")
     `(:title ,(display service) :args ,(list service))))

(defn! interface_function_items (service query)
  "Inspect registry metadata without overwriting the palette's own bindings."
  (def rows (map (info_srv service :interface_doc) (fn (record)
    (def signature (get record :interface))
    (def name (symbol (get signature 0)))
    `(:title ,(display (concat (list name) (slice signature 1)))
      :subtitle ,(or! (get record :doc) "") :aside ,(display service)
      :on_click (continue_service_call ,service ',name '())))))
  (fuzzy_match query rows (fn (row) (list (get row :title) (get row :subtitle)))))

(defn! service_call_metadata (service name)
  (def records (filter (info_srv service :interface_doc) (fn (record)
    (eq? (get (get record :interface) 0) (keyword name)))))
  (if (empty? records) (error (format "{} no longer exports {}" service name)))
  # Evaluate only the stub's initializer to inspect its real signature and types.
  (meta (eval (get (vrs/service_stub_form service (get records 0)) 2))))

(defn! invoke_service_function (service name values)
  # A palette export is already local: sending to ourselves would deadlock.
  (if (eq? (find_srv service) (self)) (apply (eval name) values)
    (call (find_srv service) (concat (list (keyword name)) values))))

(defn! continue_service_call (service name values)
  (def metadata (service_call_metadata service name))
  (def signature (get metadata :args))
  (if (eq? (len values) (len signature))
    (if (contains? '(:vrsjmp :cmd_macro) service)
      (invoke_service_function service name values)
      (start_action (vrs/command_title name metadata)
        `(call (find_srv ,service) ',(concat (list (keyword name)) values))))
    (let ((arg (get signature (len values))))
      `(:push_page :get_items service_call_items
        :args ,(list service name values) :title ,(vrs/command_title name metadata)
        :prompt ,(format "{} · {}" (display name) (display (get arg :name)))))))

(defn! service_call_expression (service name values source)
  (continue_service_call service name (push values (eval (read source)))))

(defn! service_call_items (service name values query)
  (def arg (get (get (service_call_metadata service name) :args) (len values)))
  (def type (get arg :type))
  (def providers (if (eq? type nil) '()
    (or! (get (info_srv service :entity_completions) type) '())))
  (def local_providers (if (eq? type nil) '() (entity_sources type)))
  (def available_entities (if (empty? providers) (entities type) '()))
  (map providers (fn (provider)
    (def found (try (invoke_service_function service provider '())))
    (if (list? found)
      (map found (fn (entity)
        (when! (and! (list? entity) (eq? (get entity 0) type)
                    (not? (contains? available_entities entity)))
          (set available_entities (push available_entities entity))))))))
  (if (and! (empty? providers) (empty? local_providers))
    (if (or! (eq? query "") (err? (try (read query)))) '()
      (list (make_item (str "Use " query)
        `(service_call_expression ,service ',name ',values ,query))))
    (map (fuzzy_match query available_entities) (fn (entity)
      (make_item (choice_label entity)
        `(continue_service_call ,service ',name ',(push values entity)))))))

(defn! function_items (id query)
  "Return call forms to the editor without running the selected function."
  (input_request id)
  (map (service_functions query)
    (fn (name)
      (def form (call_form name '()))
      (+ (function_item name)
         `(:on_click (finish_input ,id ',form)
           :actions ,(list
             (make_item "Fill arguments" `(fill_call ,id ',name '()))))))))

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

(defn! fill_call_value (id name arguments value)
  (fill_call id name (push arguments (literal_form value))))

(defn! fill_call_items (id name arguments query)
  (input_request id)
  (def arg (get (get (meta (eval name)) :args) (len arguments)))
  (def type (get arg :type))
  (if (empty? (entity_sources type))
    (error (format "No completions configured for {}" (display type))))
  (map (fuzzy_match query (entities type)) (fn (entity)
    (+ (make_item (entity_title entity)
         `(fill_call_value ,id ',name ',arguments ',entity))
       `(:subtitle ,(get entity :app))))))

(defn! interactive_items ()
  # Completion providers run only after selecting a command. Window commands
  # live on the Windows page; its secondary actions can fill more arguments.
  (map (filter (interactive_commands) (fn (name)
    (if (contains? '(focus_window move_window open_antinote_note) name) false
      (let ((args (get (meta (eval name)) :args)))
        (if (empty? args) true
          (not? (empty? (entity_sources (get (get args 0) :type)))))))))
    (fn (name)
      (make_item (command_title name)
        (if (eq? name 'save_page) '(save_page (active_tab))
          `(call_interactively ',name))))))

(defn! save_page (page)
  "Save to Read Later"
  (interactive :web/page)
  (if (or! (not? (list? page)) (not? (get page :url)))
    (error "No active browser page to save."))
  (def result (feedbin_call `(:feedbin_save ,(get page :url) ,(get page :title))))
  (if (err? result) result
    (if (empty? result) (error "Feedbin did not save the page. Check its connection and authentication.") result)))

(defn! call_interactively (name)
  "Fill a named command's required arguments using completion pages, then call it"
  (continue_call name '()))

(defn! continue_call (name values)
  (def signature (get (meta (eval name)) :args))
  (if (eq? (len values) (len signature))
    (if (contains? '(:vrsjmp :cmd_macro) (get (meta (eval name)) :service))
      (apply (eval name) values)
      (start_action (command_title name)
        (concat (list name) (map values literal_form))))
    (let ((arg (get signature (len values))))
      (def type (get arg :type))
      (def choices (and! (not? (eq? type nil))
                        (not? (empty? (entity_sources type)))))
      `(:push_page :get_items call_items
        :args ,(list name values)
        :title ,(command_title name)
        :prompt ,(format "{} · {}{}" (display name) (display (get arg :name))
                   (if choices "" " · e.g. \"hello\", 42, '(…)"))))))

(defn! call_expression (name values source)
  (continue_call name (push values (eval (read source)))))

(defn! call_items (name values query)
  (def arg (get (get (meta (eval name)) :args) (len values)))
  (def type (get arg :type))
  (def providers (if (eq? type nil) '() (entity_sources type)))
  (if (empty? providers)
    (if (or! (eq? query "") (err? (try (read query)))) '()
      (list (make_item (str "Use " query) `(call_expression ',name ',values ,query))))
    (map (fuzzy_match query (entities type)) (fn (entity)
      (+ (make_item (or! (get entity :title) (entity_title entity))
           `(continue_call ',name ',(push values entity)))
         `(:subtitle ,(if (eq? type :os/app)
                       (get entity :bundle_id)
                       (get entity :app))
           :aside ,(if (get entity :id) (str (get entity :id)) nil)
           :actions ,(+ (entity_actions entity)
                       (if (eq? type :os/window) (window_actions entity) '()))))))))

(defn! entity_actions (entity)
  "Secondary actions come from the commands imported into this service"
  (map (interactive_functions entity)
    (fn (name)
      (make_item (command_title name)
        `(continue_call ',name '(,entity))))))

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

(defn! display_page ()
  (+ (push_page 'display_items "Search resolutions…") '(:title "Display Resolution")))

(defn! display_items (query)
  (map (fuzzy_match query (list_alternative_resolutions) str) (fn (resolution)
    # The current marker is presentation, not part of the mode descriptor.
    (def desc (get (split " (current)" resolution) 0))
    (make_item resolution `(begin (select_resolution ,desc) :refresh)))))

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

(defn! task_items (query)
  (def index 0)
  (def title (trim_text (apply join (+ '("-")
    (filter (split "-" query) (fn (part)
      (set index (+ index 1)) (not? (eq? index 1))))))))
  (def create (if (eq? title "") '(error "Task title is empty")
                 `(things_add ,title "")))
  (+ (list (+ (make_item "Add to Things Inbox" create)
              `(:subtitle ,(if (eq? title "") nil title))))
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
             `(:subtitle ,(if excerpt excerpt (get task :notes)) :aside "Things")))))))

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
          (feedbin_call `(:feedbin_search_in ,collection ,query 20))
          '()))))
  (map (filter entries (fn (entry) (if (list? entry) (get entry :url) false))) (fn (entry)
    (def url (get entry :url))
    (def title (if (get entry :title) (get entry :title) url))
    (def host (get (split "/" url) 2))
    (def saved (get entry :created_at))
    (+ (make_item title `(open_url ,url))
       `(:subtitle ,(str (if host host url)
                       (if saved (str " · Saved " (get (split "T" saved) 0)) ""))
         :actions ,(list
           (make_item "Open in Browser" `(open_url ,url))
           (make_item "Copy URL" `(set_clipboard ,url))
           (make_item "Copy Title and URL" `(set_clipboard ,(str title "\n" url)))))))))

(def notes_cache nil)
(defn! apple_notes_page ()
  (set notes_cache (get_notes))
  (+ (push_page 'apple_notes_items "Search Apple Notes…") '(:title "Apple Notes")))

(defn! apple_notes_items (query)
  (map (fuzzy_match query notes_cache) (fn (note)
    (make_item (get note :title) `(open_note ,(get note :id))))))

(def stickies_get_cache '())
(defn! stickies_page ()
  (set stickies_get_cache (stickies_get))
  (+ (push_page 'stickies_items "Search Stickies…") '(:title "Stickies")))

(defn! stickies_items (query)
  (map (fuzzy_match query stickies_get_cache) (fn (note)
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
         `(:subtitle ,(str (if (eq? project "") "" (str project " · ")) host)
           :aside ,(str (if (get thread :unread) "Unread" "")
                       (if (get thread :modified)
                         (str (if (get thread :unread) " · " "") (get thread :modified)) "")))))))

(def tailscale_cache nil)
(defn! tailscale_page ()
  (set tailscale_cache (get_tailscale_snapshot))
  (+ (push_page 'tailscale_items "Search devices and web endpoints…")
     `(:title ,(str "Tailscale · " (get tailscale_cache :summary)))))

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
     `(:subtitle ,(str (or! (get device :dns) (get device :hostname))
                     (if address (str " · " address) ""))
       :aside ,(str (get device :os) " · "
                   (if (get device :online) "Online" "Offline")
                   (if (get device :self) " · This device" ""))
       :actions ,(+ (map copies (fn (field)
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
          `(:subtitle ,(str (get page :device) " · " (get page :url))
            :aside "Web"
            :actions ,(entity_actions (+ '(:web/page) page))))))
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
  (map (fuzzy_match query obsidian_cache) (fn (note)
    (+ (make_item (get note :title) `(open_obsidian_file ,(get note :file)))
       `(:subtitle ,(get note :file))))))

(def cloud_tabs_cache '())
(defn! cloud_tabs_page ()
  (set cloud_tabs_cache (cloud_tabs))
  (+ (push_page 'cloud_tab_items "Search Safari tabs by title, URL, or device…")
     '(:title "iCloud Tabs")))

(defn! cloud_tab_items (query)
  "Filter the page snapshot without changing its last-viewed ordering."
  (def tabs (if (eq? query "") cloud_tabs_cache
    (filter cloud_tabs_cache (fn (tab)
      (not? (empty? (fuzzy_match query (list tab) (fn (tab)
        (list (get tab :title) (get tab :url) (get tab :device))))))))))
  (map tabs (fn (tab)
    (def url (get tab :url))
    (+ (make_item (get tab :title) `(open_url ,url))
       `(:subtitle ,url :aside ,(get tab :device)
         :actions ,(+ (list (make_item "Open in Browser" `(open_url ,url))
                          (make_item "Copy URL" `(set_clipboard ,url)))
                     (entity_actions tab)))))))

(def browser_history_cache '())
(defn! browser_history_page ()
  (refresh_safari_history)
  (set browser_history_cache (get_safari_history))
  (+ (push_page 'browser_history_items "Search recent Safari history…")
     '(:title "Browser History")))

(defn! browser_history_items (query)
  (map (fuzzy_match query browser_history_cache) (fn (entry)
    (def url (get entry :url))
    (def page `(:web/page :title ,(get entry :title) :url ,url))
    (+ (make_item (get entry :title) `(open_url ,url))
       `(:subtitle ,(get (split "/" url) 2) :aside ,(get entry :visited)
         :actions ,(entity_actions page))))))

(def antinote_cache '())
(defn! antinote_page ()
  (set antinote_cache (get_antinote_notes))
  (+ (push_page 'antinote_items "Search Antinote notes…")
     '(:title "Antinote")))

(defn! antinote_items (query)
  (map (fuzzy_match query antinote_cache) (fn (note)
    (def excerpt (match_excerpt query (get note :content)))
    (+ (make_item (get note :title) `(open_antinote_note ',note))
       `(:subtitle ,(if excerpt excerpt (get note :modified))
         :aside ,(if excerpt
                  (if (get note :modified) (get (split " " (get note :modified)) 0) nil)
                  nil)
         :actions ,(list
           (make_item "Copy Note" `(set_clipboard ,(get note :content)))
           (make_item "Open Antinote" '(open_antinote))))))))

(def github_cache '())
(defn! github_page ()
  (refresh_pull_requests)
  (set github_cache (get_pull_requests))
  (+ (push_page 'github_items "Search pull requests…") '(:title "GitHub PRs")))

(defn! github_items (query)
  (map (fuzzy_match query github_cache) (fn (pr)
    (+ (make_item (get pr :title) `(open_url ,(get pr :url)))
       `(:subtitle ,(get pr :url))))))

# TODO: Nice to have "prefix-drop" for these prefixed names
(defn! macro_items (query)
  "(macro_items QUERY) - Returns markup for macro items"
  (if (not? (contains? query "macro:"))
    '()
    (+
     (map (get_macros) (fn (m) `(:title ,(get m :name)
                                 :on_click ,(get m :cmds))))
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
         (make_item "Calendar" '(open_app "Calendar"))
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
         (make_item "Palette Appearance" '(appearance_page))
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
  # Opt into input snapshots with e.g. :on_click (save_page (active_tab)).
  # Only outer-call arguments are retained; reads inside the function run again.
  # begin and other compound forms use normal execution without retry capture.
  (def result
    (if (direct_action? cmd)
      (begin
        (publish :cmd cmd)
        (def values (map (slice cmd 1) (fn (arg) (eval arg))))
        (start_action (get item :title)
          (concat (list (get cmd 0)) (map values literal_form))))
      (vrs/execute_command cmd)))
  (if (list? result)
    (if (eq? (get result 0) :push_page) result :close)
    (if (eq? result :refresh) :refresh :close)))

(defn! on_click_wait (item receiver)
  "Run an item and report completion of any background actions to the caller."
  (def before action_runs)
  (def response (on_click item))
  (def started (filter action_runs (fn (record)
    (def previous (get (filter before (fn (old) (eq? (get old :id) (get record :id)))) 0))
    (or! (not? previous) (and! (get previous :error) (not? (get record :error)))))))
  (if (empty? started) response
    (begin
      (def ids (map started (fn (record) (get record :id))))
      (set action_runs (map action_runs (fn (record)
        (if (contains? ids (get record :id))
          (if (get record :error)
            (begin
              (try (send receiver (list :vrsjmp_action (get record :id) (list :error (get record :error)))))
              record)
            (+ record (list :receiver receiver)))
          record))))
      (list :pending_actions :ids ids :response response))))

(spawn_srv! :vrsjmp :interface '(root_page get_items on_click on_click_wait enqueue_input finish_action get_ui_config set_ui_config))
