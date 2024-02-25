#!/usr/bin/env vrsctl
# vrsjmp.ll - vrsjmp commandbar
#

# TODO: Move to init.ll w/ supervision tree
(bind-srv :system_appearance)
(bind-srv :bookmarks)

(defn get_items (query)
  "Retrieve items to display"
  # TODO: Support N-ary +
  (+ (+ (favorite_items)
        (dynamic_items query))
     (get_bookmarks)))

(defn make_item (title command)
  "Create an item with TITLE and COMMAND"
  (list :title title :on_click command))

(defn dynamic_items (query)
  "Return list of dynamically generated items or empty list"
  (if (not query) '()
      (list
       (make_item "Search Perplexity"
                  (list 'open_url (format "http://perplexity.ai/?q={}" query)))
       (make_item "Search Google"
                  (list 'open_url (format "http://google.com/search?q={}" query))))))

(defn on_click (item)
  "Handle an on_click payload from item"
  (eval (get item :on_click))
  :ok)

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
   (make_item "Perplexity" '(open_url "https://www.perplexity.ai"))
   (make_item "ChatGPT" '(open_url "https://chat.openai.com"))
   (make_item "Slack" '(open_app "Slack"))
   (make_item "Soulver" '(open_app "Soulver 3"))
   (make_item "Restart vrsd" '(exec "pkill" "-ax" "vrsd"))
   (make_item "Toggle Darkmode" '(toggle_darkmode))

   (make_item "Bookmarks - Add" '(bookmark_active_tab))
   (make_item "Bookmarks - Clear" '(clear_bookmarks))
   ))


(spawn-srv :vrsjmp :interface '(get_items on_click))
