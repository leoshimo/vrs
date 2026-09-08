;;; vrs-choose-tests.el --- Native chooser tests -*- lexical-binding: t; -*-

(require 'ert)
(require 'vrs-choose)

(defmacro vrs-test--select (choices &rest body)
  "Run BODY selecting CHOICES through the normal completion table interface."
  (declare (indent 1))
  `(let ((choices ,choices))
     (cl-letf (((symbol-function 'completing-read)
                (lambda (_prompt table predicate require-match &rest _)
                  (should require-match)
                  (should (memq (completion-metadata-get
                                 (completion-metadata "" table predicate) 'category)
                                '(vrs-value vrs-function)))
                  (let* ((candidates (all-completions "" table predicate))
                         (choice (pop choices)))
                    (cond ((eq choice 'quit) (signal 'quit nil))
                          ((integerp choice) (nth choice candidates))
                          (t (or (cl-find-if (lambda (candidate)
                                              (string-match-p choice candidate)) candidates)
                                 (ert-fail (format "No match for %s in %S" choice candidates)))))))))
       ,@body)))

(defun vrs-test--request (source)
  "Evaluate SOURCE through the real session without a chooser transaction."
  (let ((output (generate-new-buffer " *VRS test result*"))
        (errors (generate-new-buffer " *VRS test errors*")))
    (unwind-protect
        (progn
          (with-temp-buffer
            (insert source)
            (should (= 0 (vrs--run-region (point-min) (point-max)
                                          vrs-vrsctl-command output errors "compact" nil 90))))
          (with-current-buffer output (string-remove-suffix "\n" (buffer-string))))
      (kill-buffer output) (kill-buffer errors))))

(ert-deftest vrs-native-selection-keeps-identical-labels-distinct ()
  (with-temp-buffer
    (vrs-mode)
    (insert "(values)")
    (cl-letf (((symbol-function 'vrs--chooser-data)
               (lambda (_source)
                 '(("Same" "'(:test/item :title \"Same\" :id 1)")
                   ("Same" "'(:test/item :title \"Same\" :id 2)")))))
      (vrs-test--select '(1) (vrs-choose-value)))
    (should (equal (buffer-string) "'(:test/item :title \"Same\" :id 2)"))))

(ert-deftest vrs-native-retains-settings-and-exact-multiform-region ()
  (with-temp-buffer
    (vrs-mode)
    (insert "(begin\n  (def x 1)\n  (list x) # trailing comment\n  :after)")
    (setq-local vrs-vrsctl-command "custom-vrsctl")
    (setq-local vrs-result-width 60)
    (goto-char 10)
    (set-mark (point))
    (search-forward "comment")
    (setq mark-active t)
    (let ((transient-mark-mode t))
      (cl-letf (((symbol-function 'vrs--run-region)
                 (lambda (start end command output _errors format raw width)
                   (should (equal command "custom-vrsctl"))
                   (should (equal (list format raw width) '("compact" nil 60)))
                   (should (equal (buffer-substring-no-properties start end)
                                  "(vrs/editor_choices (begin\n(def x 1)\n  (list x) # trailing comment\n) false)"))
                   (with-current-buffer output (insert "((\"1\" \"1\"))\n"))
                   0)))
        (vrs-test--select '(0) (vrs-choose-value))))
    (should (equal (buffer-string) "(begin\n  1\n  :after)"))))

(ert-deftest vrs-native-cancellation-and-evaluation-failure-keep-source ()
  (dolist (failure '(quit error))
    (with-temp-buffer
      (vrs-mode) (insert "(values)")
      (cl-letf (((symbol-function 'vrs--run-region)
                 (lambda (_start _end _command output errors &rest _)
                   (with-current-buffer output (insert "((\"a\" \"'a\"))\n"))
                   (with-current-buffer errors (insert "expected failure"))
                   (if (eq failure 'error) 1 0))))
        (vrs-test--select '(quit)
          (if (eq failure 'quit)
              (should (condition-case nil (progn (vrs-choose-value) nil) (quit t)))
            (should-error (vrs-choose-value) :type 'user-error))))
      (should (equal (buffer-string) "(values)")))))

(ert-deftest vrs-native-pending-edits-and-killed-buffers-prevent-commit ()
  (dolist (change '(edit kill))
    (let ((buffer (generate-new-buffer " *VRS target*")))
      (unwind-protect
          (with-current-buffer buffer
            (vrs-mode) (insert "(values)")
            (cl-letf (((symbol-function 'vrs--run-region)
                       (lambda (_start _end _command output &rest _)
                         (with-current-buffer output (insert "((\"a\" \"'a\"))\n"))
                         (if (eq change 'kill) (kill-buffer buffer)
                           (with-current-buffer buffer (insert " changed")))
                         0)))
              (should-error (vrs-choose-value) :type 'user-error))
            (when (buffer-live-p buffer)
              (with-current-buffer buffer (should (equal (buffer-string) "(values) changed")))))
        (when (buffer-live-p buffer) (kill-buffer buffer))))))

(ert-deftest vrs-native-edits-during-completion-prevent-execution ()
  (with-temp-buffer
    (vrs-mode) (insert "(entity)")
    (let (requests)
      (cl-letf (((symbol-function 'vrs--chooser-data)
                 (lambda (source)
                   (push source requests)
                   (if (string-prefix-p "(vrs/editor_value" source)
                       '("An entity" "'(:test/item :id 1)")
                     '(("act" "Action" "session" "(act entity)" (("entity" ":test/item")))))))
                ((symbol-function 'completing-read)
                 (lambda (&rest _) (insert " edited") "Action"))
                ((symbol-function 'vrs--chooser-request)
                 (lambda (&rest _) (ert-fail "Must not execute after source edits"))))
        (should-error (vrs-execute-action) :type 'user-error))
      (should (= 2 (length requests)))
      (should (equal (buffer-string) "(entity) edited")))))

(ert-deftest vrs-native-browse-cancels-after-partial-argument-filling ()
  (with-temp-buffer
    (vrs-mode) (insert "before ")
    (cl-letf (((symbol-function 'vrs--chooser-data)
               (lambda (source)
                 (if (equal source "(vrs/editor_functions)")
                     '(("act" "Action" ":example" "(act one two)"
                        (("one" ":example/item") ("two" ":example/item"))))
                   '(("First" "'(:example/item :id 1)"))))))
      (vrs-test--select '(0 0 quit)
        (should (condition-case nil (progn (vrs-browse-functions-minibuffer t) nil) (quit t)))))
    (should (equal (buffer-string) "before "))))

(ert-deftest vrs-native-packet-does-not-interpret-lyric-source ()
  (with-temp-buffer
    (vrs-mode)
    (let ((vrs--chooser-target (list (current-buffer) (buffer-chars-modified-tick) 1 1)))
      (dolist (packet '("((\"title\" \"'name#suffix\"))" "((\"title\" \"'path\\\\part\"))"))
        (cl-letf (((symbol-function 'vrs--chooser-request) (lambda (&rest _) packet)))
          (should (stringp (cadar (vrs--chooser-data "nil"))))))
      (dolist (packet '("((title nil))" "#1=(#1#)" "() extra"))
        (cl-letf (((symbol-function 'vrs--chooser-request) (lambda (&rest _) packet)))
          (should-error (vrs--chooser-data "nil")))))))

(ert-deftest vrs-native-values-and-fields-with-test-runtime ()
  (skip-unless (getenv "VRS_TEST_VRSCTL"))
  (let ((vrs-vrsctl-command (getenv "VRS_TEST_VRSCTL")))
    (unwind-protect
        (progn
          (dolist (value '("nil" "false" "true" "42" "()" "hello" "name#suffix"
                           "path\\part" "@name" "(unquote @name)"
                           "(quote (a (b c)))" "(missing_function :argument)"
                           "\"one\\n\\\"two\\\"\\\\東京\"" "\"\""
                           "(:os/process :pid 42 :nested (hello (a b)))"))
            (with-temp-buffer
              (vrs-mode) (insert (format "(list '%s)" value))
              (vrs-test--select '(0) (vrs-choose-value))
              (should (equal (vrs-test--request (format "(eq? %s '%s)" (buffer-string) value)) "true"))))
          (dolist (entry '(("'(:record :nested (missing_function))" "nested" "'(missing_function)")
                           ("'(:title \"東京\" :enabled false)" "enabled" "false")
                           ("'(:record :symbol hello)" "symbol" "'hello")))
            (with-temp-buffer
              (vrs-mode) (insert (car entry))
              (vrs-test--select (list (nth 1 entry)) (vrs-choose-field))
              (should (equal (buffer-string) (nth 2 entry)))))
          (with-temp-buffer
            (vrs-mode) (insert "'((:title \"Same\" :id 1) (:title \"Same\" :id 2))")
            (vrs-test--select '(1) (vrs-choose-value))
            (should (equal (buffer-string) "'(:title \"Same\" :id 2)")))
          (dolist (source '("'()" "42" "(list (ref))" "(list (fn () nil))" "(missing_function)"))
            (with-temp-buffer
              (vrs-mode) (insert source)
              (should-error (vrs-choose-value) :type 'user-error)
              (should (equal (buffer-string) source))))
          (dolist (source '("'(:x)" "'(:x 1 :bad)" "'()" "42"))
            (with-temp-buffer
              (vrs-mode) (insert source)
              (should-error (vrs-choose-field) :type 'user-error)
              (should (equal (buffer-string) source)))))
      (vrs--close-session vrs-vrsctl-command))))

(defconst vrs-test--native-fixture
  "(def native_hits '())
   (def native_queries 0)
   (defn! native_objects ()
     (set native_queries (+ native_queries 1))
     '((:test/object :title \"Same\" :id 1) (:test/object :title \"Same\" :id 2)))
   (defn! native_destinations () '((:test/dest :title \"Here\" :path (a b))))
   (defn! native_copy (object destination)
     \"Copy PID\" (interactive :test/object :test/dest)
     (set native_hits (push native_hits (list object destination))) :copied)
   (defn! native_source (object expression)
     \"Source argument\" (interactive :test/object :test/source)
     (set native_hits (push native_hits (list object expression))) :sourced)
   (defn! native_plain (expression) expression)
   (defn! native_status () (list native_hits native_queries))
   (defn! native_failure (object)
     \"Failure\" (interactive :test/object)
     '(:exit 1 :stderr \"fixture failed\"))
   (set_entity_completions :test/object 'native_objects)
   (set_entity_completions :test/dest 'native_destinations)
   (spawn_srv! :native_fixture
     :interface '(native_objects native_destinations native_copy native_source
                  native_plain native_status native_failure))")

(ert-deftest vrs-native-discovery-fill-and-explicit-execution-with-test-runtime ()
  (skip-unless (getenv "VRS_TEST_VRSCTL"))
  (let ((vrs-vrsctl-command (getenv "VRS_TEST_VRSCTL")))
    (unwind-protect
        (progn
          (vrs-test--request vrs-test--native-fixture)
          ;; Start a fresh client to test imported, not locally defined, metadata.
          (vrs--close-session vrs-vrsctl-command)
          (vrs-test--request "(bind_srv :native_fixture)")
          (should (equal (vrs-test--request "(err? (try (find_srv :vrsjmp)))") "true"))
          (with-temp-buffer
            (vrs-mode) (insert "(begin\n  \n  :after)") (goto-char 10)
            (vrs-test--select '("native_copy") (vrs-browse-functions-minibuffer nil))
            (should (equal (buffer-string) "(begin\n  (native_copy object destination)\n  :after)")))
          (should (equal (vrs-test--request "(native_status)") "(() 0)"))
          (with-temp-buffer
            (vrs-mode)
            (vrs-test--select '("native_copy" 1 0) (vrs-browse-functions-minibuffer t))
            (should (equal (buffer-string)
                           "(native_copy '(:test/object :title \"Same\" :id 2) '(:test/dest :title \"Here\" :path (a b)))")))
          (should (equal (vrs-test--request "(native_status)") "(() 1)"))
          (with-temp-buffer
            (vrs-mode) (insert "(get (native_objects) 1)")
            (vrs-test--select '("Copy PID" 0) (vrs-act-on-value))
            (should (equal (buffer-string)
                           "(native_copy '(:test/object :title \"Same\" :id 2) '(:test/dest :title \"Here\" :path (a b)))")))
          (should (equal (vrs-test--request "(native_status)") "(() 2)"))
          ;; Manual expressions are parsed and inserted, never evaluated here.
          (with-temp-buffer
            (vrs-mode)
            (cl-letf (((symbol-function 'read-string) (lambda (&rest _) "(native_objects)")))
              (vrs-test--select '("native_plain") (vrs-browse-functions-minibuffer t)))
            (should (equal (buffer-string) "(native_plain (native_objects))")))
          (should (equal (vrs-test--request "(native_status)") "(() 2)"))
          (with-temp-buffer
            (vrs-mode) (insert "'(:test/object :id 9)")
            (cl-letf (((symbol-function 'read-string) (lambda (&rest _) "(native_objects)")))
              (vrs-test--select '("Source argument") (vrs-act-on-value)))
            (should (equal (buffer-string) "(native_source '(:test/object :id 9) (native_objects))")))
          (with-temp-buffer
            (vrs-mode) (insert "'(:test/object :id 9)")
            (cl-letf (((symbol-function 'read-string) (lambda (&rest _) "(unfinished")))
              (vrs-test--select '("Source argument")
                (should-error (vrs-execute-action) :type 'user-error)))
            (should (equal (buffer-string) "'(:test/object :id 9)")))
          (with-temp-buffer
            (vrs-mode) (insert "'(:test/object :id 9)")
            (vrs-test--select '("Copy PID" quit)
              (should (condition-case nil (progn (vrs-execute-action) nil) (quit t))))
            (should (equal (buffer-string) "'(:test/object :id 9)")))
          (with-temp-buffer
            (vrs-mode) (insert "'(:unhandled/entity :id 9)")
            (should-error (vrs-act-on-value) :type 'user-error)
            (should (equal (buffer-string) "'(:unhandled/entity :id 9)")))
          (should (equal (vrs-test--request "(native_status)") "(() 2)"))
          (vrs-test--request "(subscribe :cmd)")
          (with-temp-buffer
            (vrs-mode) (insert "(get (native_objects) 0)")
            (vrs-test--select '("Copy PID" 0) (vrs-execute-action))
            (should (equal (buffer-string) "(get (native_objects) 0)")))
          (should (equal (vrs-test--request "(native_status)")
                         "((((:test/object :title \"Same\" :id 1) (:test/dest :title \"Here\" :path (a b)))) 3)"))
          (should (equal (vrs-test--request "(recv)")
                         "(:topic_updated :cmd (native_copy '(:test/object :title \"Same\" :id 1) '(:test/dest :title \"Here\" :path (a b))))"))
          (with-current-buffer "*VRS Result*" (should (equal (buffer-string) ":copied\n")))
          (with-temp-buffer
            (vrs-mode) (insert "'(:test/object :id 1)")
            (vrs-test--select '("Failure")
              (should-error (vrs-execute-action) :type 'user-error))
            (should (equal (buffer-string) "'(:test/object :id 1)"))))
      (vrs--close-session vrs-vrsctl-command))))

(ert-deftest vrs-native-cancels-a-pending-request-with-test-runtime ()
  (skip-unless (getenv "VRS_TEST_VRSCTL"))
  (let ((vrs-vrsctl-command (getenv "VRS_TEST_VRSCTL")))
    (unwind-protect
        (with-temp-buffer
          (vrs-mode) (insert "(recv)")
          (let ((timer (run-at-time 0.1 nil (lambda () (setq quit-flag t)))))
            (unwind-protect
                (should (condition-case nil (progn (vrs-choose-value) nil) (quit t)))
              (cancel-timer timer)))
          (should-not (gethash vrs-vrsctl-command vrs--sessions))
          (should (equal (buffer-string) "(recv)"))
          (erase-buffer) (insert "'(hello)")
          (vrs-test--select '(0) (vrs-choose-value))
          (should (equal (buffer-string) "'hello")))
      (vrs--close-session vrs-vrsctl-command))))

(provide 'vrs-choose-tests)
;;; vrs-choose-tests.el ends here
