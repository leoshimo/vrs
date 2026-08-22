# Automatic history records boundary submissions, not nested evaluations.
(defn! vrs/history_should_capture? (expression)
  (not? (and! (list? expression)
              (not? (empty? expression))
              (eq? (get expression 0) 'history))))

(defn! vrs/history_capture_submission (request)
  (def expression (vrs/history_submission request))
  (if (vrs/history_should_capture? expression)
    (history_append expression)))
