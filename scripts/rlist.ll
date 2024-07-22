#!/usr/bin/env vrsctl
# rlist.ll - Reading List
#

(bind_srv :os_browser)

# TODO: Nice-to-have is defining CRUD resource service via macro

(def rlist_path "~/Dropbox/rlist.ll")

(def (:id id :rlist rlist) (begin
    (def res (try (def (:id _ :rlist _) (fread rlist_path))))
    (if (ok? res) res '(:id 0 :rlist ()))))

(defn save_rlist ()
  "(save_rlist) - Save current rlist to filesystem"
  (spawn (fn () (fdump rlist_path (list :id id :rlist rlist)))))

(defn get_rlist ()
  "(get_rlist) - Get all items in reading list"
  rlist)

(defn add_rlist (title url)
  "(add_rlist TITLE URL) - Add item with TITLE and URL to reading list"
  (set rlist (push rlist (list :id id :rlist :title title :url url)))
  (set id (+ id 1))
  (save_rlist)
  (publish :rlist_event :updated_rlist)
  :ok)

(defn remove_rlist (id)
  "(remove_rlist ID) - Remove item with ID from reading list"
  (set rlist (filter rlist (fn (it) (not? (contains? it id)))))
  (save_rlist)
  (publish :rlist_event :updated_rlist))

(defn clear_rlist ()
  "(clear_rlist) - Clear all reading list items"
  (set rlist '())
  (save_rlist)
  (publish :rlist_event :updated_rlist)
  :ok)

(defn add_rlist_active_tab ()
  "(add_rlist_active_tab) - Add current active page of browser to reading list"
  (if (def (:title title :url url) (active_tab))
    (add_rlist title url)))

(spawn_srv :rlist
   :interface '(get_rlist add_rlist remove_rlist clear_rlist add_rlist_active_tab))
