# Source-embedded debug tools: leave a useful observation beside the code.
# Run this file, or evaluate it with C-c C-c in Emacs vrs-mode.
# Inspect with `vrsctl dbg --all` or `vrsctl dbg --web`.

(defn! twice (x)
  (+ x x))

(dbg!
  (def numbers '(2 3 5))
  (map numbers twice))

# A second invocation stays separate in the same shared recording.
(dbg! (map '(8 13) twice))
