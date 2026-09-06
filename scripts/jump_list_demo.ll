#!/usr/bin/env vrsctl
# jump_list_demo.ll - Jump List (Demo)
#

# Initial State
(def jump_list '())

# Function Interfaces
(defn get_jump_list ()
  "(get_jump_list) - Get all items in jump list"
  jump_list)

(defn add_jump_list (title url)
  "(add_jump_list TITLE URL) - Add item with TITLE and URL to jump list"
  (set jump_list (push jump_list (list :jump_list :title title :url url)))
  (publish :jump_list_event (list :updated_jump_list jump_list))
  :ok)

(defn clear_jump_list ()
  "(clear_jump_list) - Clear all jump list items"
  (set jump_list '())
  (publish :jump_list_event (list :updated_jump_list jump_list))
  :ok)

# Fork service
(spawn_srv! :jump_list :interface '(get_jump_list add_jump_list clear_jump_list))






















# DEMO: Integrate Browser
# (bind_srv :os_browser)

# (defn add_jump_list_active_tab ()
#   "(add_jump_list_active_tab) - Add current browser tab to jump list"
    # TODO: Fill Me!
#   :ok)

# (spawn_srv! :jump_list :interface
#    '(get_jump_list add_jump_list clear_jump_list add_jump_list_active_tab))
