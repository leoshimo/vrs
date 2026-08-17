#!/usr/bin/env vrsctl
# obsidian.ll - Obsidian
#

(defn obsidian_vault_path ()
  "Use the most recently opened vault from Obsidian's own configuration"
  (def result (exec "jq" "-r"
    "[.vaults[]] | sort_by(.ts) | reverse | (map(select(.open)) + .) | .[0].path // empty"
    (shell_expand "~/Library/Application Support/obsidian/obsidian.json")))
  (if (not? (eq? (get result :exit) 0))
    (error (str "Could not read Obsidian configuration: " (get result :stderr))))
  (def path (get (split "\n" (get result :stdout)) 0))
  (if (eq? path "") (error "Open a vault in Obsidian first"))
  path)

(defn get_obsidian_files ()
  "(get_obsidian_files) - Get list of files in Obsidian"
  (def vault_path (obsidian_vault_path))
  (def result
    (exec "bash" "-seuo" "pipefail" "--" vault_path
          :stdin """
          cd "$1"
          find . -type d \( -name .obsidian -o -name .trash -o -name .git \) -prune -o -type f -iname '*.md' -print0 |
            jq -Rs 'split("\u0000")[:-1]'
          """))
  (if (eq? (get result :exit) 0)
    (map (decode :json (get result :stdout))
         (fn (f) (list :title (get (split "/" f) -1) :file (str vault_path "/" f))))
    (error (str "Could not read Obsidian vault: " (get result :stderr)))))

(defn open_obsidian_file (file)
  "(open_obsidian_file FILE) - Opens given item in obsidian"
  (def result (exec "jq" "-nr" "--arg" "file" file
    "\"obsidian://open?\" + (if $file | startswith(\"/\") then \"path=\" else \"file=\" end) + ($file | @uri)"))
  (if (not? (eq? (get result :exit) 0)) (error (get result :stderr)))
  (exec "open" (get (split "\n" (get result :stdout)) 0)))

(spawn_srv! :obsidian :interface '(get_obsidian_files open_obsidian_file))
