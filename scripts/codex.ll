#!/usr/bin/env vrsctl
# Read the desktop app's cached catalog (including remote hosts), never its logs.
# TODO: Replace these private storage formats with a desktop API when available.

(defn! get_codex_threads ()
  "Read recent Codex threads and their unread state from the desktop app"
  (def result (exec "bash" "-seuo" "pipefail" :stdin """
    codex_dir="${CODEX_HOME:-$HOME/.codex}"
    catalog=""
    # Release and development builds use different catalog names. If both
    # exist, use the one most recently maintained by the app.
    for candidate in "$codex_dir/sqlite/codex.db" "$codex_dir/sqlite/codex-dev.db"; do
      if [ -f "$candidate" ] && { [ -z "$catalog" ] || [ "$candidate" -nt "$catalog" ]; }; then
        catalog="$candidate"
      fi
    done
    if [ -z "$catalog" ]; then
      echo "Open Codex once to populate its thread catalog." >&2
      exit 1
    fi

    unread='{}'
    if [ -f "$codex_dir/.codex-global-state.json" ]; then
      unread="$(jq -c '."electron-persisted-atom-state"."unread-thread-ids-by-host-v1" // {}' "$codex_dir/.codex-global-state.json")"
    fi

    sqlite3 -readonly -json -cmd '.timeout 1000' "$catalog" "
      SELECT thread_id AS id, display_title AS title, host_id AS host,
             coalesce(cwd, '') AS cwd,
             strftime('%Y-%m-%d %H:%M',
               coalesce(nullif(source_recency_at, 0), source_updated_at),
               'unixepoch', 'localtime') AS modified
        FROM local_thread_catalog
       WHERE missing_candidate = 0
         AND source_kind IN ('cli', 'vscode', 'appServer', 'exec', 'unknown')
       ORDER BY coalesce(nullif(source_recency_at, 0), source_updated_at) DESC,
                source_created_at DESC, thread_id, host_id
    " | jq -s --argjson unread "$unread" '
      (.[0] // []) | map(. as $thread | . + {
        unread: (($unread[$thread.host] // []) | index($thread.id) != null),
        url: ("codex://threads/" + (.id | @uri))
      })'
    """))
  (if (not? (eq? (get result :exit) 0))
    (error (str "Could not read Codex threads: " (get result :stderr))))
  (map (decode :json (get result :stdout))
    (fn (thread) (+ '(:codex/thread) thread))))

(defn! open_codex_thread (thread)
  "Open a thread in the Codex desktop app"
  (def result (exec "open" (get thread :url)))
  (if (not? (eq? (get result :exit) 0))
    (error (str "Could not open Codex thread: " (get result :stderr)))))

(spawn_srv! :codex :interface '(get_codex_threads open_codex_thread))
