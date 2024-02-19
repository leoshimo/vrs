#!/usr/bin/env vrsctl
# launcher.ll - A launcher service

(def items '())

(defn get_items ()
  items)

(defn add_item (title cmd)
  (set items (push items (mk_item title cmd)))
  :ok)

(defn mk_item (title cmd)
  (list :title title :on_click cmd))

(add_item "Browser" '(open_app "Safari"))
(add_item "Things" '(open_app "Things3"))
(add_item "Terminal" '(open_app "Alacritty"))
(add_item "Messages" '(open_app "Messages"))
(add_item "Mail" '(open_app "Spark"))
(add_item "Cal" '(open_app "Notion Calendar"))
(add_item "Zulip" '(open_app "Zulip"))
(add_item "Checkins" '(open_url "https://recurse.zulipchat.com/#narrow/stream/27333-alumni-checkins/topic/Leo.20Shimonaka"))
(add_item "X" '(open_url "https://www.x.com"))
(add_item "GitHub - vrs" '(open_url "https://www.github.com/leoshimo/vrs"))
(add_item "Downloads" '(open_file "~/Downloads"))
(add_item "Dropbox" '(open_file "~/Dropbox"))
(add_item "Kindle" '(open_app "Kindle"))
(add_item "Send to Kindle" '(open_url "https://www.amazon.com/gp/sendtokindle"))
(add_item "RC - Presentations" '(open_url "https://presentations.recurse.com"))
(add_item "AWS Console" '(open_url "http://console.aws.amazon.com"))
(add_item "Perplexity" '(open_url "https://www.perplexity.ai"))
(add_item "ChatGPT" '(open_url "https://chat.openai.com"))
(add_item "Slack" '(open_app "Slack"))
(add_item "Soulver" '(open_app "Soulver 3"))
(add_item "Restart vrsd" '(exec "pkill" "-ax" "vrsd"))

# TODO: Need better DX around calling service functions (exported via sexp, but symbol needs reexporting in client process)
(add_item "Toggle Darkmode" '(call (find-srv :system_appearance) '(:toggle_darkmode)))

(spawn-srv :launcher
   :interface '(get_items add_item))

