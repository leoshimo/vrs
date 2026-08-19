#!/usr/bin/env vrsctl
# Send this local file with: vrsctl --node minato ./scripts/demo-remote.ll
# Change :version to 2 and send it again. Existing host_probe bindings follow
# the replacement process; the previous process's in-memory state is discarded.

(defn! host_probe ()
  "Return the hosting node and this service revision."
  (list :node (node_name) :version 1))

(spawn_srv! :host_probe :interface '(host_probe))
