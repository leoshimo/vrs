;;; vrs-session-tests.el --- Editor session tests -*- lexical-binding: t; -*-
(require 'ert)
(require 'vrs-mode)

(defmacro vrs-test--sessions (&rest body)
  `(let ((vrs--sessions (make-hash-table :test #'eq))
         (vrs--session-list nil) (vrs--last-session nil)
         (vrs--local-nodes (make-hash-table :test #'equal)))
     (unwind-protect (progn ,@body)
       (dolist (session vrs--session-list) (vrs--close-session session)))))

(ert-deftest vrs-session-names-are-optional-and-unique-per-node ()
  (vrs-test--sessions
   (let ((first (vrs--fresh-session "vrsctl" nil)))
     (should (equal (vrs--editor-session-name first) "1"))
     (should (equal (vrs--editor-session-name (vrs--fresh-session "vrsctl" nil "")) "2"))
     (should-error (vrs--fresh-session "vrsctl" nil "1") :type 'user-error)
     (should (vrs--fresh-session "vrsctl" "beta" "1"))
     (should-error (vrs--fresh-session "vrsctl" "beta" "bad/name") :type 'user-error))))

(ert-deftest vrs-buffers-inherit-last-session-but-retain-existing-selections ()
  (vrs-test--sessions
   (let ((a (generate-new-buffer " *VRS a*"))
         (b (generate-new-buffer " *VRS b*")))
     (unwind-protect
         (progn
           (with-current-buffer a (vrs-mode))
           (with-current-buffer b
             (vrs-mode)
             (setq vrs--selected-session (vrs--get-session vrs-vrsctl-command "beta" "scratch"))
             (vrs--remember-session))
           (with-temp-buffer
             (vrs-mode)
             (should (eq (vrs--current-session) vrs--last-session)))
           (with-current-buffer a
             (should-not (vrs--editor-session-node (vrs--current-session)))
             (vrs--remember-session))
           (with-temp-buffer (vrs-mode) (should-not (vrs--editor-session-node (vrs--current-session)))))
       (kill-buffer a) (kill-buffer b)))))

(ert-deftest vrs-session-renaming-keeps-identity-and-rejects-duplicates ()
  (vrs-test--sessions
   (with-temp-buffer
     (vrs-mode)
     (let ((session (vrs--current-session)))
       (vrs-rename-session "work")
       (should (eq session (vrs--current-session)))
       (should (equal (vrs--session-label session) "local/work"))
       (vrs--fresh-session vrs-vrsctl-command nil "other")
       (should-error (vrs-rename-session "other") :type 'user-error)
       (should (equal (vrs--editor-session-name session) "work"))))))

(ert-deftest vrs-session-picker-reuses-and-implicitly-creates ()
  (vrs-test--sessions
   (with-temp-buffer
     (vrs-mode)
     (puthash vrs-vrsctl-command "alpha" vrs--local-nodes)
     (let ((choice "beta/scratch") calls)
       (cl-letf (((symbol-function 'vrs--discover-nodes) (lambda () '("alpha" "alpha" "beta")))
                 ((symbol-function 'completing-read) (lambda (&rest _) choice))
                 ((symbol-function 'vrs--session-evaluate) (lambda (session _) (push session calls) "nil")))
		(vrs-select-session nil)
		(let ((remote (vrs--current-session)))
		  (should (equal (vrs--session-label remote) choice))
		  (setq choice "alpha") (vrs-select-session nil)
		  (should-not (vrs--editor-session-node (vrs--current-session)))
		  (setq choice "beta/scratch") (vrs-select-session nil)
		  (should (eq remote (vrs--current-session))))
		(should (= 3 (length calls))))))))

(ert-deftest vrs-session-chooser-pins-its-originating-session ()
  (vrs-test--sessions
   (with-temp-buffer
     (vrs-mode) (insert "(list 1)")
     (setq vrs--selected-session (vrs--get-session vrs-vrsctl-command "beta"))
     (let ((target vrs--selected-session))
       (cl-letf (((symbol-function 'vrs--run-region)
                  (lambda (_start _end session output _errors &rest _)
                    (should (eq target session))
                    (with-current-buffer output (insert "1\n")) 0)))
		(vrs--chooser (cons (point-min) (point-max))
			      (lambda ()
				(setq vrs--selected-session (vrs--get-session vrs-vrsctl-command nil))
				(should (equal (vrs--chooser-request "1") "1")))))))))

(ert-deftest vrs-debugger-command-preserves-node-and-socket ()
  (vrs-test--sessions
   (with-temp-buffer
     (let ((vrs-vrsctl-command "vrsctl --socket /tmp/example.sock"))
       (vrs-mode)
       (setq vrs--selected-session (vrs--get-session vrs-vrsctl-command "beta"))
       (cl-letf (((symbol-function 'make-process)
                  (lambda (&rest args)
                    (should (equal (car (last (plist-get args :command)))
                                   "exec vrsctl --socket /tmp/example.sock dbg --web --node beta")))))
		(vrs-open-debugger))))))

(ert-deftest vrs-source-imports-prepare-only-the-selected-session-with-test-runtime ()
  (skip-unless (getenv "VRS_TEST_VRSCTL"))
  (vrs-test--sessions
   (let* ((vrs-vrsctl-command (getenv "VRS_TEST_VRSCTL"))
          (admin (vrs--fresh-session vrs-vrsctl-command nil))
          (target (vrs--fresh-session vrs-vrsctl-command nil)))
     (vrs--session-evaluate
      admin
      (concat "(defn! hint_value () :first) (defn! hint_local () :service) "
              "(register_entity_source :hint_value 'hint_value) "
              "(register_entity_source :hint_local 'hint_local) "
              "(spawn_srv! :editor_hint_first :interface '(hint_value hint_local)) "
              "(defn! hint_value () :second) "
              "(spawn_srv! :editor_hint_second :interface '(hint_value))"))
     (vrs--session-evaluate target "(def hint_local nil) (def runs 0) (defn! nested () (hint_value))")
     (with-temp-buffer
       (vrs-mode)
       (setq vrs--selected-session target)
       ;; A running service alone does not make its functions available.
       (insert "(hint_value)")
       (should-error (vrs-eval-last-sexp t) :type 'user-error)
       (erase-buffer)
       (let ((prefix (concat "(bind_srv :editor_hint_first)\n"
                             "(error \"Do not evaluate surrounding source\")\n"
                             "(bind_srv :editor_hint_second)\n")))
         (insert prefix "(begin (set runs (+ runs 1)) (list (nested) hint_local runs))")
         (vrs-eval-last-sexp t)
         (should (equal (buffer-string) (concat prefix "'(:second nil 1)")))
         (should (equal (vrs--session-evaluate target "(entity_sources :hint_value)") "(hint_value)"))
         (should (equal (vrs--session-evaluate target "(entity_sources :hint_local)") "()"))
         ;; Repeated preparation preserves a later session override.
         (vrs--session-evaluate target "(defn! hint_value () :override)")
         (erase-buffer)
         (insert prefix "(nested)")
         (vrs-eval-last-sexp t)
         (should (equal (buffer-string) (concat prefix ":override"))))
       (erase-buffer)
       (insert "(bind_srv :editor_hint_missing)\n(set runs 99)")
       (should-error (vrs-eval-last-sexp t) :type 'user-error)
       (should (equal (buffer-string) "(bind_srv :editor_hint_missing)\n(set runs 99)"))
       (should (equal (vrs--session-evaluate target "runs") "1"))
       (should (equal (vrs--session-evaluate
                       (vrs--fresh-session vrs-vrsctl-command nil)
                       "(err? (try hint_value))") "true"))))))

(ert-deftest vrs-source-import-cache-follows-the-connection-with-test-runtime ()
  (skip-unless (getenv "VRS_TEST_VRSCTL"))
  (vrs-test--sessions
   (let* ((vrs-vrsctl-command (getenv "VRS_TEST_VRSCTL"))
          (admin (vrs--fresh-session vrs-vrsctl-command nil))
          (target (vrs--fresh-session vrs-vrsctl-command nil))
          (evaluate (symbol-function 'vrs--session-evaluate))
          requests)
     (vrs--session-evaluate
      admin
      (concat "(defn! cached_first () 1) (defn! cached_second () 2) "
              "(spawn_srv! :editor_cache_first :interface '(cached_first)) "
              "(spawn_srv! :editor_cache_second :interface '(cached_second))"))
     (cl-letf (((symbol-function 'vrs--session-evaluate)
                (lambda (session source)
                  (when (string-prefix-p "(vrs/editor_bind_services " source)
                    (push source requests))
                  (funcall evaluate session source))))
       (cl-labels ((run (session imports expression)
                     ;; Each call uses a different buffer sharing SESSION.
                     (with-temp-buffer
                       (vrs-mode)
                       (setq vrs--selected-session session)
                       (insert imports "\n" expression)
                       (vrs-eval-last-sexp t))))
         (let* ((first "(bind_srv :editor_cache_first)")
                (both (concat first "\n(bind_srv :editor_cache_second)")))
           (run target first "(cached_first)")
           (run target first "(cached_first)")
           (should (= (length requests) 1))
           (run target both "(+ (cached_first) (cached_second))")
           (should (= (length requests) 2))
           (should (equal (car requests) "(vrs/editor_bind_services '(:editor_cache_second))"))
           (with-temp-buffer
             (vrs-mode)
             (setq vrs--selected-session target)
             (vrs-reset-session))
           (run target both "(+ (cached_first) (cached_second))")
           (should (= (length requests) 3))
           (should (equal (car requests) "(vrs/editor_bind_services '(:editor_cache_first :editor_cache_second))"))
           (delete-process (gethash target vrs--sessions))
           (run target both "(+ (cached_first) (cached_second))")
           (should (= (length requests) 4))
           (run (vrs--fresh-session vrs-vrsctl-command nil) both "(cached_second)")
           (should (= (length requests) 5))
           ;; A failed import must be retried, not cached as successful.
           (dotimes (_ 2)
             (should-error (run target "(bind_srv :editor_cache_missing)" "nil") :type 'user-error))
           (should (= (length requests) 7))))))))

(ert-deftest vrs-local-and-remote-sessions-with-test-runtime ()
  (skip-unless (getenv "VRS_TEST_REMOTE_VRSCTL"))
  (vrs-test--sessions
   (let ((vrs-vrsctl-command (getenv "VRS_TEST_REMOTE_VRSCTL")))
     (with-temp-buffer
       (vrs-mode)
       (should (equal (vrs--discover-nodes) '("alpha" "alpha" "beta")))
       (let ((local (vrs--current-session))
             (remote (vrs--get-session vrs-vrsctl-command "beta"))
             (other (vrs--fresh-session vrs-vrsctl-command "beta")))
         (should (equal (vrs--session-evaluate local "(def answer 11)") "11"))
         (should (equal (vrs--session-evaluate remote "(def answer 42)") "42"))
         (should (equal (vrs--session-evaluate remote "answer") "42"))
         (should (equal (vrs--session-evaluate local "answer") "11"))
         (should (equal (vrs--session-evaluate other "(err? (try answer))") "true"))
         (setq vrs--selected-session remote)
         (let ((process (gethash remote vrs--sessions)))
           (vrs-rename-session "work")
           (should (eq process (gethash remote vrs--sessions)))
           (should (equal (vrs--mode-name) "VRS[beta/work]"))
           (insert "(+ answer 1)")
           (vrs-eval-last-sexp t)
           (should (equal (buffer-string) "43"))
           (erase-buffer)
           (insert "(bind_srv :host_probe)\n(list (host_probe) answer)")
           (vrs-eval-last-sexp t)
           (should (equal (buffer-string) "(bind_srv :host_probe)\n'((\"beta\" 2) 42)"))
           (dolist (untouched (list local other))
             (should (equal (vrs--session-evaluate untouched "(err? (try host_probe))") "true")))
           (vrs-reset-session)
           (should-not (eq process (gethash remote vrs--sessions))))
         (should (equal (vrs--session-evaluate remote "(err? (try answer))") "true"))
         (should (equal (vrs--session-evaluate local "answer") "11"))
         (delete-process (gethash remote vrs--sessions))
         (should (equal (vrs--mode-name) "VRS[beta/work!]"))
         (should (equal (vrs--session-evaluate remote "(node_name)") "\"beta\""))
         (should-error (vrs--session-evaluate (vrs--get-session vrs-vrsctl-command "missing") "(def answer 99)")
                       :type 'user-error)
         (should (equal (vrs--session-evaluate local "answer") "11")))))))

(provide 'vrs-session-tests)
