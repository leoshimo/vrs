#!/usr/bin/env vrsctl
# obsidian.ll - Obsidian
#

(def vault_path (shell_expand "~/obsidian/main"))

(defn get_obsidian_files ()
  "(get_obsidian_files) - Get list of files in Obsidian"
  (def (:ok result) (exec "bash" "-c" (format "find {} -iname '*.md' | sed -E s:{}/::g" vault_path vault_path)))
  # TODO: List unpacking? Can't use (join " - " (split "/" f)) b.c. join is (join args...)
  (map (split "\n" result) (fn (f) (list :title (get (split "/" f) -1) :file f))))

(defn open_obsidian_file (file)
  "(open_obsidian_file FILE) - Opens given item in obsidian"
  (exec "open" (format "obsidian://open?file={}" file)))

(spawn_srv :obsidian :interface '(get_obsidian_files open_obsidian_file))
