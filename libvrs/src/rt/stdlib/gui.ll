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
                  :title "Browse service functions" :prompt "Search functions or services…")))

(defn! vrsjmp_choose (values)
  "(vrsjmp_choose VALUES) - Open vrsjmp and return one element of VALUES."
  (vrs/choose_page values :values))

(defn! vrsjmp_choose_field (record)
  "(vrsjmp_choose_field RECORD) - Open vrsjmp and return a field's value. Accepts keyword/value records and tagged entities."
  (if (not? (list? record)) (error "Expected a keyword/value record"))
  (def pairs (try (vrs/field_pairs record)))
  # A tagged entity has one leading keyword before its key/value pairs.
  (if (err? pairs)
    (if (keyword? (get record 0))
      (set pairs (vrs/field_pairs (slice record 1)))
      (error "Expected a keyword/value record")))
  (vrs/choose_page pairs :fields))

(defn! vrs/field_pairs (fields)
  (if (empty? fields) '()
    (begin
      (if (or! (eq? (len fields) 1) (not? (keyword? (get fields 0))))
        (error "Expected a keyword/value record"))
      (concat (list (list (get fields 0) (get fields 1)))
              (vrs/field_pairs (slice fields 2))))))

(defn! vrs/choose_page (values mode)
  (if (not? (list? values)) (error "Expected a list of choices"))
  (if (empty? values) (error "No values to choose from"))
  (request_input
    (list :push_page :get_items 'choose_items :args (list values mode)
          :title (if (eq? mode :fields) "Choose a field" "Choose a value")
          :prompt (if (eq? mode :fields) "Search fields…" "Search values…"))))
