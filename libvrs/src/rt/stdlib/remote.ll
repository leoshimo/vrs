# The body remains source until it reaches the destination process.
# The Rust runtime, not this transformer, checkpoints the destination registry
# and awaits application on the caller's node before releasing a result.
# See rt/remote.rs for acknowledgement ordering and failure limits.
(defmacro remote (node & body)
  "(remote! NODE FORMS...) - Evaluate a block on NODE and return after its completed registrations are visible on the caller's node. No evaluation deadline; readiness is registration, not health."
  `(eval_remote ,node '(begin ,@body)))
