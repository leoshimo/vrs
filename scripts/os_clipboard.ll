#!/usr/bin/env vrsctl
# os_clipboard - OS Clipboard
#

(defn get_clipboard ()
  "(get_clipboard) - Get contents of clipboard"
  (get (exec "pbpaste") :stdout))

(defn set_clipboard (contents)
  "(set_clipboard CONTENTS) - Set contents of clipboard"
  (exec "pbcopy" :stdin contents))

(spawn_srv :os_clipboard :interface '(get_clipboard set_clipboard))
