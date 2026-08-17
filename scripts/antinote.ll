#!/usr/bin/env vrsctl
# Antinote 2: read notes through SQLite, never modify its database.

(def antinote_path
  (shell_expand "~/Library/Containers/com.chabomakers.Antinote/Data/Library/Application Support/cd-v1-notes.sqlite"))

(defn! get_antinote_notes ()
  "Read recent Antinote notes, excluding private, locked, archived and deleted notes"
  (def result
    (exec "sqlite3" "-readonly" "-json" "-cmd" ".timeout 1000" antinote_path
      "SELECT upper(substr(hex(ZID),1,8) || '-' || substr(hex(ZID),9,4) || '-' ||
                    substr(hex(ZID),13,4) || '-' || substr(hex(ZID),17,4) || '-' ||
                    substr(hex(ZID),21,12)) AS id,
              substr(trim(ZCONTENT, char(9) || char(10) || char(13) || ' '), 1,
                     instr(trim(ZCONTENT, char(9) || char(10) || char(13) || ' ') || char(10), char(10)) - 1) AS title,
              ZCONTENT AS content,
              datetime(ZLASTMODIFIED + 978307200, 'unixepoch', 'localtime') AS modified
         FROM ZNOTE
        WHERE length(trim(coalesce(ZCONTENT, ''), char(9) || char(10) || char(13) || ' ')) > 0
          AND coalesce(ZSOFTDELETED, 0) = 0
          AND coalesce(ZISPRIVATE, 0) = 0
          AND coalesce(ZISLOCKED, 0) = 0
          AND coalesce(ZISARCHIVED, 0) = 0
        ORDER BY ZLASTMODIFIED DESC LIMIT 500"))
  (if (not? (eq? (get result :exit) 0))
    (error (str "Could not read Antinote notes: " (get result :stderr))))
  (map (if (eq? (get result :stdout) "") '() (decode :json (get result :stdout)))
    (fn (note) (+ '(:antinote/note) note))))

(defn! open_antinote ()
  "Open Antinote without selecting or promoting a note"
  (exec "open" "-a" "Antinote"))

(defn! open_antinote_note (note)
  "Open Antinote Note"
  (interactive :antinote/note)
  # Antinote's supported selection URL also promotes the note to the front.
  # TODO: Replace with plain open if a supported/non-promoting route is found.
  (exec "open" (str "antinote://x-callback-url/promoteAndOpen?noteId=" (get note :id))))

(set_entity_completions :antinote/note 'get_antinote_notes)
(spawn_srv! :antinote :interface '(get_antinote_notes open_antinote open_antinote_note))
