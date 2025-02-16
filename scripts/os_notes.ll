#!/usr/bin/env vrsctl
# os_notes.ll - Apple Notes
#

(def notes '())

# Adapted from https://github.com/raycast/extensions/blob/main/extensions/apple-notes/src/useNotes.ts
(defn load_notes ()
  "(refresh_notes) - Refreshes in-memory contents from notes DB"
  (def output (get (exec "sqlite3" (shell_expand "~/Library/Group Containers/group.com.apple.notes/NoteStore.sqlite")
        "SELECT '(:id \"' || id || '\" :title \"' || title || '\")'
       FROM (
        SELECT note.zidentifier as id,
            note.ztitle1 AS title,
            datetime (note.zmodificationdate1 + 978307200, 'unixepoch') AS modifiedAt
        FROM
            ziccloudsyncingobject AS note
            INNER JOIN ziccloudsyncingobject AS folder ON note.zfolder = folder.z_pk
            LEFT JOIN ziccloudsyncingobject AS acc ON note.zaccount4 = acc.z_pk
            LEFT JOIN z_metadata AS zmd ON 1 = 1
        WHERE
            note.ztitle1 IS NOT NULL
            AND note.zmodificationdate1 IS NOT NULL
            AND note.zmarkedfordeletion != 1
            AND folder.zmarkedfordeletion != 1
            AND folder.ztitle2 IS NOT \"Recently Deleted\"
        ORDER BY
            note.zmodificationdate1 DESC)
        LIMIT 200") 1))
    (read (format "({})" output)))

(defn refresh_notes ()
  (set notes (load_notes)))

(defn get_notes ()
  "(get_notes) - Returns notes"
  (refresh_notes)
  notes)

(defn open_note (id)
  "(open_note) - Open note with given ID"
  (def url (format "applenotes:/note/{}" id))
  (exec "open" url))

(defn create_note (title body)
  "(create_note TITLE BODY OPEN) - Create note with TITLE and BODY"
  (def contents (format "
    <div>
    <h1>{}</h1><br/>

    {}
    </div>" (if title title "") (if body body "")))

  (exec "osascript" "-e" "tell application \"Notes\""
        "-e" "activate"
        "-e" "set newNote to make new note"
        "-e" (format "set body of newNote to \"{}\"" contents)
        "-e" "set selection to newNote"
        "-e" "show newNote"
        "-e" "end tell"))

(refresh_notes)
(spawn_srv :os_notes :interface '(open_note get_notes create_note))
