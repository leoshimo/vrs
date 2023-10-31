#!/usr/bin/env vrsctl
# launcher.ll - A launcher service

(def items '())

(defn get_items ()
  items)

(defn add_item (title cmd)
  (set items (push items (mk_item title cmd))))

(defn mk_item (title cmd)
  (list :title title :on_click cmd))

(add_item "Browser" '(open_app "Safari"))
(add_item "Things" '(open_app "Things3"))
(add_item "Terminal" '(open_app "Alacritty"))
(add_item "Messages" '(open_app "Messages"))
(add_item "Mail" '(open_app "Spark"))
(add_item "Cron" '(open_app "Cron"))
(add_item "Zulip" '(open_app "Zulip"))
(add_item "Twitter" '(open_url "https://www.twitter.com"))
(add_item "GitHub - vrs" '(open_url "https://www.github.com/leoshimo/vrs"))
(add_item "Downloads" '(open_file "~/Downloads"))
(add_item "Send to Kindle" '(open_url "https://www.amazon.com/gp/sendtokindle"))

#
# Whoops service mgmt is broken
# (srv :name :launcher
#      :export '(get_items add_item))
#
(loop
   (def req (recv))
   (let ((r (get req 0))
         (src (get req 1))
         (msg (get req 2)))

     (def resp
        (if (eq? (get msg 0) :add_item)
        (begin
            (add_item (get msg 1) (get msg 2))
            :ok)
        (if (eq? (get msg 0) :get_items)
            (get_items)
            '(:err "Unrecognized message"))))

     (send src (list r resp))
   ))
