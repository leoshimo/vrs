# The body remains source until it reaches the destination process.
(defmacro remote (node & body)
  "(remote! NODE FORMS...) - Evaluate a block on NODE and return its result."
  `(eval_remote ,node '(begin ,@body)))
