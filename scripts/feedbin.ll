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

(defn feedbin_search (query count)
  "(feedbin_search QUERY COUNT) - Return up to COUNT indexed Feedbin entries matching QUERY"
  (decode_feedbin_result
    (try (exec "feedbinctl" "search" query "--limit" (display count)))))

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

(spawn_srv :feedbin :interface '(feedbin_entries feedbin_search))
