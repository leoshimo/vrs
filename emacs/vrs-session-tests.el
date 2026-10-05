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
