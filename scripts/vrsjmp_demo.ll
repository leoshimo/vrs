#!/usr/bin/env vrsctl
# vrsjmp_demo.ll - Slim vrsjmp for demos
#

(bind_srv :system_appearance)
(bind_srv :nl_shell)
(bind_srv :os_screencap)
(bind_srv :jump_list)

(defn get_items (callback args query)
  (apply (eval callback) (push args query)))

(defn begin_interaction ()
  '(:push_page :get_items root_items :prompt "Search"))

(defn root_items (query)
  "Retrieve items to display"
  (+ (fuzzy_match query (+ (favorite_items) (jump_list_items)) display)
     (query_items query)))

(defn make_item (title command)
  "Create an item with TITLE and COMMAND"
  (list :title title :on_click command))

(defn query_items (query)
  "Return a dynamic list of item for current query"
  (if (not? query) '() (list
       # DEMO: Integrate Do It
       # (make_item "Do It" `(codegen_exec ,query))
       (make_item "Search Perplexity" `(open_url ,(format "http://perplexity.ai/?q={}&copilot=true" query)))
       (make_item "Search Google" `(open_url ,(format "http://google.com/search?q={}" query)))
    )))

(defn jump_list_items ()
  "(jump_list_items) - Retrieve item markup for jump list"
  (map (get_jump_list) (fn (b)
    (make_item (format "Jump List - {}" (get b :title))
               `(open_url ,(get b :url))))))





























(defn favorite_items ()
  "Returns list of static vrsjmp items"
  (list
   (make_item "Browser" '(open_app "Safari"))
   (make_item "Terminal" '(open_app "Alacritty"))
   (make_item "Cal" '(open_app "Notion Calendar"))

   (make_item "GitHub - vrs" '(open_url "https://www.github.com/leoshimo/vrs"))
   (make_item "GitHub - eventkitcli" '(open_url "https://github.com/leoshimo/eventkitcli"))

   # DEMO: Integrate Jump List
   # (make_item "Add to Jump List" '(add_jump_list_active_tab))
   # (make_item "Clear Jump List" '(clear_jump_list))

   # DEMO: Reify Interaction
   # (begin (bind_srv :jump_list)
   #        (get (get_jump_list) -1))

   (make_item "Screen Capture" '(start_screencap))
   ))

(defn on_click (item)
  "Handle an on_click payload from item"
  (def cmd (get item :on_click))
  (def result (eval cmd))
  (if (list? result)
    (if (eq? (get result 0) :push_page) result :close)
    :close))

(spawn_srv :vrsjmp :interface '(get_items on_click))
