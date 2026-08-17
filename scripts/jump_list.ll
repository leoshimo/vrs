#!/usr/bin/env vrsctl
# jump_list.ll - Small persistent link list for demos, separate from Feedbin
#

(bind_srv :os_browser)

# TODO: Nice-to-have is defining CRUD resource service via macro

# Keep the legacy data path and record key; renaming the service must not move
# or discard an existing user's saved links. This service is now demo-only.
(def jump_list_path "~/Dropbox/rlist.ll")

(def id 0)
(def jump_list '())

(defn load_jump_list ()
  (def res (try (def (:id _ :rlist _) (fread jump_list_path))))
  (if (ok? res) (begin
                 (set id (get res :id))
                 (set jump_list (get res :rlist)))))

(defn save_jump_list ()
  "(save_jump_list) - Save current jump_list to filesystem"
  (spawn (fn () (fdump jump_list_path (list :id id :rlist jump_list)))))

(defn get_jump_list ()
  "(get_jump_list) - Get all items in jump list"
  (load_jump_list)
  jump_list)

(defn add_jump_list (title url)
  "(add_jump_list TITLE URL) - Add item with TITLE and URL to jump list"
  (set jump_list (push jump_list (list :id id :jump_list :title title :url url)))
  (set id (+ id 1))
  (save_jump_list)
  (publish :jump_list_event :updated_jump_list)
  :ok)

(defn remove_jump_list (id)
  "(remove_jump_list ID) - Remove item with ID from jump list"
  (set jump_list (filter jump_list (fn (it) (not? (contains? it id)))))
  (save_jump_list)
  (publish :jump_list_event :updated_jump_list))

(defn clear_jump_list ()
  "(clear_jump_list) - Clear all jump list items"
  (set jump_list '())
  (save_jump_list)
  (publish :jump_list_event :updated_jump_list)
  :ok)

(defn add_jump_list_active_tab ()
  "(add_jump_list_active_tab) - Add current active page of browser to jump list"
  (if (def (:title title :url url) (active_tab))
    (add_jump_list title url)))

(spawn_srv! :jump_list
   :interface '(get_jump_list add_jump_list remove_jump_list clear_jump_list add_jump_list_active_tab))
