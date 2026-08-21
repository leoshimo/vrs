#!/usr/bin/env vrsctl
# Oura background indexer backed by ouractl.

(def parent (self))
(def indexer
  (spawn
    (fn ()
      (try (kill (find_srv :oura)))
      (register :oura :overwrite)
      (send parent (list :oura_ready (self)))
      (loop
        (let ((result (try (exec "ouractl" "index"))))
          (if (err? result)
            (dbg (list :oura_index :error result))
            (if (eq? (get result :exit) 0)
              (dbg (list :oura_index :ok (get result :stdout)))
              (dbg (list :oura_index :error (get result :stderr))))))
        (sleep 3600)))))
(recv (list :oura_ready indexer))

indexer
