#!/usr/bin/env vrsctl
# rlist.ll - Reading List
#

(bind_srv :os_browser)

(def rlist_path "~/rlist.ll")
(def rlist (begin
  (def res (try (fread rlist_path)))
  (if (ok? res) res '())))

(defn save_rlist ()
  "(save_rlist) - Save current rlist to filesystem"
  (spawn (fn () (fdump rlist_path rlist))))

(defn get_rlist ()
  "(get_rlist) - Get all items in reading list"
  rlist)

(defn add_rlist (title url)
  "(add_rlist TITLE URL) - Add item with TITLE and URL to reading list"
  (set rlist (push rlist (list :rlist :title title :url url)))
  (save_rlist)
  :ok)

(defn clear_rlist ()
  "(clear_rlist) - Clear all reading list items"
  (set rlist '())
  (save_rlist)
  :ok)

(defn add_rlist_active_tab ()
  "(add_rlist_active_tab) - Add current active page of browser to reading list"
  (if (def (:title title :url url) (active_tab))
    (add_rlist title url)))

(spawn_srv :rlist
   :interface '(get_rlist add_rlist clear_rlist add_rlist_active_tab))
