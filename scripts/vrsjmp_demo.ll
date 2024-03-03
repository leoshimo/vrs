#!/usr/bin/env vrsctl
# vrsjmp_demo.ll - Slim vrsjmp for demos
#

(bind_srv :system_appearance)
(bind_srv :nl_shell)
(bind_srv :os_screencap)
(bind_srv :rlist)

(defn get_items (query)
  "Retrieve items to display"
  (+ (favorite_items)
     (rlist_items)
     (query_items query)))

(defn make_item (title command)
  "Create an item with TITLE and COMMAND"
  (list :title title :on_click command))

(defn query_items (query)
  "Return a dynamic list of item for current query"
  (if (not? query) '() (list
       # DEMO: Integrate Do It
       # (make_item "Do It" (list 'do_it query))
       (make_item "Search Perplexity" (list 'open_url (format "http://perplexity.ai/?q={}&copilot=true" query)))
       (make_item "Search Google" (list 'open_url (format "http://google.com/search?q={}" query)))
    )))

(defn rlist_items ()
  "(rlist_items) - Retrieve item markup for reading list"
  (map (get_rlist) (fn (b)
    (make_item (format "Reading List - {}" (get b :title))
               (list 'open_url (get b :url))))))

(defn favorite_items ()
  "Returns list of static vrsjmp items"
  (list
   (make_item "Browser" '(open_app "Safari"))
   (make_item "Terminal" '(open_app "Alacritty"))
   (make_item "Cal" '(open_app "Notion Calendar"))

   (make_item "GitHub - vrs" '(open_url "https://www.github.com/leoshimo/vrs"))
   (make_item "GitHub - eventkitcli" '(open_url "https://github.com/leoshimo/eventkitcli"))

   # DEMO: Integrate Reading List
   # (make_item "Add to Reading List" '(add_rlist_active_tab))
   # (make_item "Clear Reading List" '(clear_rlist))

   # DEMO: Reify Interaction
   # (begin (bind_srv :rlist)
   #        (get (get_rlist) -1))

   (make_item "Screen Capture" '(start_screencap))
   ))

(defn on_click (item)
  "Handle an on_click payload from item"
  (def cmd (get item :on_click))
  (def res (try (eval cmd)))
  (if (err? res)
    (notify "Encountered error" (format "{}" err))))

(spawn_srv :vrsjmp :interface '(get_items on_click))
