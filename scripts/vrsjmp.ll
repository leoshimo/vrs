#!/usr/bin/env vrsctl
# vrsjmp.ll - vrsjmp commandbar
#

(defn get_items (query)
  "Retrieve items to display"
  (+ (bookmarks)
     (dynamic_items query)))

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

(defn bookmarks ()
  "Returns list of static bookmarks"
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

   # TODO: Need better DX around calling service functions
   # Service exports needs re-import in client process - Global Namespace?
   (make_item "Toggle Darkmode" '(call (find-srv :system_appearance) '(:toggle_darkmode)))
   ))


(spawn-srv :vrsjmp :interface '(get_items))
