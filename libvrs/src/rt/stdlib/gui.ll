# These helpers run in the requesting process, never in the GUI service loop.
(defn! show_gui ()
  "(show_gui) - Ask a running vrsjmp to show itself; its service chooses the page."
  (publish :vrsjmp :show))

(defn! request_input (page)
  "(request_input PAGE) - Queue a GUI page and wait for its value. The page callback receives a request ID before its :args."
  (def request (display (ref)))
  (def queued (call (find_srv :vrsjmp) (list :enqueue_input request (self) page)))
  (if (err? queued) (error (display queued)))
  (vrs/wait_gui request))

(defn! vrs/wait_gui (request)
  (def response (recv (list request '_ '_)))
  (if (eq? (get response 1) :pending) (vrs/wait_gui request)
    (if (eq? (get response 1) :ok) (get response 2)
      (error "GUI input cancelled"))))

(defn! vrsjmp_browse_functions ()
  "(vrsjmp_browse_functions) - Open vrsjmp to browse bound service functions and return a call form, optionally filling arguments. Does not execute the selected call."
  (request_input '(:push_page :get_items function_items :args ()
                  :title "Browse service functions" :prompt "Find a service function…")))
