#!/usr/bin/env vrsctl
# rlist_demo.ll - Reading List (Demo)
#

# Initial State
(def rlist '())

# Function Interfaces
(defn get_rlist ()
  "(get_rlist) - Get all items in reading list"
  rlist)

(defn add_rlist (title url)
  "(add_rlist TITLE URL) - Add item with TITLE and URL to reading list"
  (set rlist (push rlist (list :rlist :title title :url url)))
  (publish :rlist_event (list :updated_rlist rlist))
  :ok)

(defn clear_rlist ()
  "(clear_rlist) - Clear all reading list items"
  (set rlist '())
  (publish :rlist_event (list :updated_rlist rlist))
  :ok)

# Fork service
(spawn_srv :rlist :interface '(get_rlist add_rlist clear_rlist))






















# DEMO: Integrate Browser
# (bind_srv :os_browser)

# (defn add_rlist_active_tab ()
#   "(add_rlist_active_tab) - Add current browser tab to reading list"
    # TODO: Fill Me!
#   :ok)

# (spawn_srv :rlist :interface
#    '(get_rlist add_rlist clear_rlist add_rlist_active_tab))
