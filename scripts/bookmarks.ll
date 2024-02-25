#!/usr/bin/env vrsctl
# bookmarks.ll - Bookmarking
#

(bind-srv :os_browser)

(def bookmarks '())

(defn get_bookmarks ()
  bookmarks)

(defn bookmark_active_tab ()
  "Bookmark active tab, if any"
  (match (active_tab)
    (nil nil)
    ((:title title :url url) (set bookmarks (push bookmarks (list :title title :on_click (list 'open_url url)))))))

(defn clear_bookmarks ()
  "Clear all bookmarks"
  (set bookmarks '()))

(spawn-srv :bookmarks :interface '(get_bookmarks bookmark_active_tab clear_bookmarks))
