#!/usr/bin/env vrsctl
# os_clipboard - OS Clipboard
#

(defn get_clipboard ()
  "(get_clipboard) - Get contents of clipboard"
  (get (exec "pbpaste") 1))

(defn set_clipboard (contents)
  "(set_clipboard CONTENTS) - Set contents of clipboard"
  # TODO: Support passing stdin w/o intermediate script
  # See https://github.com/leoshimo/dots/blob/main/bin/pbcopy_shim
  (exec "pbcopy_shim" contents))

(spawn_srv :os_clipboard :interface '(get_clipboard set_clipboard))
