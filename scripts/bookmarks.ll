#!/usr/bin/env vrsctl
# bookmarks.ll - Bookmarking
#

(bind_srv :os_browser)

(def bookmarks_path "~/bookmarks.ll")
(def bookmarks (begin
  (def res (try (fread bookmarks_path)))
  (if (ok? res) res '())))

(defn save_bookmarks ()
  "(save_bookmarks) - Save current bookmarks to filesystem"
  (spawn (fn () (fdump bookmarks_path bookmarks))))

(defn get_bookmarks ()
  "(get_bookmarks) - Get all bookmarks"
  bookmarks)

(defn add_bookmark (title url)
  "(add_bookmark TITLE URL) - Add bookmark with TITLE and URL"
  (set bookmarks (push bookmarks
                       (list :bookmark :title title :url url)))
  (save_bookmarks)
  :ok)

(defn clear_bookmarks ()
  "(clear_bookmarks) - Clear all bookmarks"
  (set bookmarks '())
  (save_bookmarks)
  :ok)

(defn add_bookmark_active_tab ()
  "(add_bookmark_active_tab) - Bookmark active tab, if any"
  (def tab (active_tab))
  (if (def (:title title :url url) (active_tab))
    (add_bookmark title url)))

(spawn_srv :bookmarks
   :interface '(get_bookmarks add_bookmark clear_bookmarks add_bookmark_active_tab))
