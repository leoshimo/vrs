#!/usr/bin/env vrsctl
# bookmarks.ll - Bookmarking
#

(bind_srv :os_browser)

(def bookmarks_path "~/bookmarks.ll")
(def bookmarks (begin
  (def res (try (fread bookmarks_path)))
  (if (ok? res) res '())))

(defn save_bookmarks ()
  "(save_bookmarks) - Save current bookmarks to file"
  (spawn (fn () (fdump bookmarks_path bookmarks))))

(defn get_bookmarks ()
  bookmarks)

(defn bookmark_active_tab ()
  "Bookmark active tab, if any"
  (match (active_tab)
    (nil nil)
    ((:title title :url url) (begin
                              (set bookmarks (push bookmarks
                                                   (list :title (format "Bookmarks - {}" title)
                                                         :on_click (list 'open_url url))))
                              (save_bookmarks)))))

(defn clear_bookmarks ()
  "Clear all bookmarks"
  (set bookmarks '())
  (save_bookmarks))

(spawn_srv :bookmarks :interface '(get_bookmarks bookmark_active_tab clear_bookmarks))
