#!/usr/bin/env vrsctl
# Feedbin index and search service backed by feedbinctl.

(defn decode_feedbin_result (result)
  (if (err? result)
    '()
    (if (eq? (get result :exit) 0)
      (let ((decoded (try (decode :json (get result :stdout)))))
        (if (ok? decoded) decoded '()))
      '())))

(defn feedbin_entries (count)
  "(feedbin_entries COUNT) - Return the COUNT most recent indexed Feedbin entries"
  (decode_feedbin_result
    (try (exec "feedbinctl" "entries" "--limit" (display count)))))

(defn feedbin_collections ()
  "(feedbin_collections) - Return indexed Feedbin feeds and saved searches"
  (decode_feedbin_result (try (exec "feedbinctl" "collections"))))

(defn feedbin_entries_from (collection count)
  "(feedbin_entries_from COLLECTION COUNT) - Return entries from feed:ID or saved-search:ID"
  (decode_feedbin_result
    (try (exec "feedbinctl" "entries"
               "--collection" collection
               "--limit" (display count)))))

(defn feedbin_search (query count)
  "(feedbin_search QUERY COUNT) - Return up to COUNT indexed Feedbin entries matching QUERY"
  (decode_feedbin_result
    (try (exec "feedbinctl" "search" query "--limit" (display count)))))

(defn feedbin_search_in (collection query count)
  "(feedbin_search_in COLLECTION QUERY COUNT) - Search within feed:ID or saved-search:ID"
  (decode_feedbin_result
    (try (exec "feedbinctl" "search" query
               "--collection" collection
               "--limit" (display count)))))

(defn feedbin_saved_pages ()
  "(feedbin_saved_pages) - Return the 20 most recently saved Feedbin Pages"
  (decode_feedbin_result
    (try (exec "feedbinctl" "pages" "list" "--limit" "20"))))

(defn feedbin_save (url)
  "(feedbin_save URL) - Save a URL to Feedbin Pages and return its entry"
  (decode_feedbin_result
    (try (exec "feedbinctl" "pages" "add" url))))

# Keep indexing outside the service process so long-running network activity
# cannot delay entries or search calls. Naming the indexer ensures re-evaluating
# this script replaces the previous loop instead of leaving an orphan behind.
(def parent (self))
(def indexer
  (spawn
    (fn ()
      (try (kill (find_srv :feedbin_indexer)))
      (register :feedbin_indexer :overwrite)
      (send parent (list :feedbin_indexer_ready (self)))
      (loop
        (let ((result (try (exec "feedbinctl" "index"))))
          (if (err? result)
            (dbg (list :feedbin_index :error result))
            (if (eq? (get result :exit) 0)
              (dbg (list :feedbin_index :ok (get result :stdout)))
              (dbg (list :feedbin_index :error (get result :stderr))))))
        (sleep 900)))))
(recv (list :feedbin_indexer_ready indexer))

(spawn_srv :feedbin
  :interface '(feedbin_collections
               feedbin_entries
               feedbin_entries_from
               feedbin_search
               feedbin_search_in
               feedbin_saved_pages
               feedbin_save))
