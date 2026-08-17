#!/usr/bin/env vrsctl
# Local, best-effort context. No clipboard tricks, global entity store, or polling.
# TODO: Add a dedicated macOS context CLI, called here rather than by the GUI.
# Use native window/Accessibility APIs with a short deadline and partial results;
# accept an explicit window ID/PID, or resolve the last non-launcher window.
# Yabai `recent` is best-effort; window stacking order is not focus history.
# Selected text may need capture before focus changes. Keep this enrichment off
# the palette-opening path; vrsjmp.ll should turn its entities into candidates.
(bind_srv :os_browser)

(defn! context_window_query (arguments)
  (def result (try (apply exec arguments)))
  (if (list? result)
    (if (eq? (get result :exit) 0)
      (let ((window (try (decode :json (get result :stdout)))))
        (if (list? window) window nil))
      nil)
    nil))

(defn! launcher_window? (window)
  (if (eq? (get window :app) "vrsjmp") true
    (eq? (get window :title) "vrsjmp")))

(defn! context_window ()
  (def window (context_window_query '("yabai" "-m" "query" "--windows" "--window")))
  (if (list? window)
    (if (launcher_window? window)
      (let ((recent (context_window_query '("yabai" "-m" "query" "--windows" "--window" "recent"))))
        (if (list? recent) (if (launcher_window? recent) nil recent) nil))
      window)
    nil))

(defn! selected_text (app)
  "Best-effort AX selection; an unfocused app may no longer expose it"
  (def result (try (exec "osascript" "-e" """
    on run argv
      try
        with timeout of 1 seconds
          tell application "System Events"
            tell process (item 1 of argv)
              set focusedElement to value of attribute "AXFocusedUIElement"
              return value of attribute "AXSelectedText" of focusedElement
            end tell
          end tell
        end timeout
      on error
        return ""
      end try
    end run
    """ app)))
  (if (list? result)
    (if (eq? (get result :exit) 0)
      # osascript appends a newline even for an empty selection. Preserve actual
      # multiline selections, including blank lines, apart from that terminator.
      (let ((lines (split "\n" (get result :stdout))))
        (def index 0)
        (apply join (+ '("\n") (filter lines (fn (line)
          (set index (+ index 1))
          (not? (eq? index (len lines))))))))
      "")
    ""))

(defn! get_context_for_window (window_id)
  "Snapshot the originating window, browser page, and accessible selected text"
  (def window (if window_id
    (context_window_query (list "yabai" "-m" "query" "--windows" "--window" (str window_id)))
    (context_window)))
  (if (not? (list? window)) '()
    (begin
      (def objects (list (list :os/window :id (get window :id)
                              :app (get window :app) :title (get window :title))))
      (def tab (try (active_tab_for_app (get window :app))))
      (if (list? tab)
        (if (get tab :url) (set objects (push objects (+ '(:web/page) tab)))))
      (def selection (selected_text (get window :app)))
      (if (not? (empty? selection))
        (set objects (push objects (list :text :title "Selected Text" :value selection))))
      objects)))

(defn! get_context ()
  (get_context_for_window nil))

(set_entity_completions :text 'get_context)
(spawn_srv! :os_context :interface '(get_context get_context_for_window))
