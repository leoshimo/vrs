;;; lyric-mode-tests.el --- Tests for lyric-mode -*- lexical-binding: t; -*-

(require 'ert)
(require 'lyric-mode)
(require 'cl-lib)

(ert-deftest lyric-evaluation-selects-the-form-at-a-closing-paren ()
  (dolist (source '("(ls_srv)" "'((1 2) (3 4))" "`(a ,(get x 0))"
                    "(srv! :test :interface '())"))
    (with-temp-buffer
      (insert source)
      (lyric-mode)
      (dolist (position (list (1- (point-max)) (point-max)))
        (goto-char position)
        (should (equal (lyric--last-sexp-source) source)))))
  (with-temp-buffer
    (insert "(begin (ls_srv))")
    (lyric-mode)
    (goto-char (- (point-max) 2))
    (should (equal (lyric--last-sexp-source) "(ls_srv)"))
    (forward-char)
    (should (equal (lyric--last-sexp-source) "(begin (ls_srv))"))))

(ert-deftest lyric-evaluation-process-keeps-stdout-and-stderr-separate ()
  (with-temp-buffer
    (insert "exact source: \"東京\"\n")
    (let ((output (generate-new-buffer " *Lyric test output*"))
          (errors (generate-new-buffer " *Lyric test errors*")))
      (unwind-protect
          (progn
            (should (= 7 (lyric--run-region
                          (point-min) (point-max)
                          "sh -c 'cat; printf diagnostic >&2; exit 7'" output errors)))
            (should (equal (with-current-buffer output (buffer-string))
                           "exact source: \"東京\"\n"))
            (should (equal (with-current-buffer errors (buffer-string)) "diagnostic")))
        (kill-buffer output)
        (kill-buffer errors)))))

(ert-deftest lyric-evaluation-quit-kills-the-client-and-preserves-source ()
  (with-temp-buffer
    (insert "(srv! :test :interface '())")
    (lyric-mode)
    (goto-char (1- (point-max)))
    (let ((lyric-vrsctl-command "sleep 30 #")
          (make-real-process (symbol-function 'make-process))
          child cancelled)
      (cl-letf (((symbol-function 'make-process)
                 (lambda (&rest args)
                   (setq child (apply make-real-process args))))
                ((symbol-function 'accept-process-output)
                 (lambda (&rest _) (signal 'quit nil))))
        (condition-case nil
            (lyric-eval-last-sexp t)
          (quit (setq cancelled t))))
      (should cancelled)
      (should child)
      (should-not (process-live-p child))
      (should (equal (buffer-string) "(srv! :test :interface '())")))))

(ert-deftest lyric-evaluation-with-test-runtime ()
  "Optional end-to-end evaluation, enabled by the terminal test harness."
  (skip-unless (getenv "LYRIC_TEST_VRSCTL"))
  (let ((lyric-vrsctl-command (getenv "LYRIC_TEST_VRSCTL"))
        (lyric-result-width 10))
    (with-temp-buffer
      (insert "(pretty '((1 2) (3 4)) 10)")
      (lyric-mode)
      (goto-char (point-max))
      (lyric-eval-last-sexp nil)
      (with-current-buffer "*Lyric Result*"
        (should (equal (buffer-string) "((1 2)\n (3 4))\n")))
      (erase-buffer)
      (insert "(list? (ls_srv))")
      (goto-char (1- (point-max)))
      (lyric-eval-last-sexp t)
      (should (equal (buffer-string) "true"))
      (erase-buffer)
      (insert "'((1 2) (3 4))")
      (lyric-eval-last-sexp t)
      (should (equal (buffer-string) "((1 2)\n (3 4))"))
      (erase-buffer)
      (insert "\"one\\n\\\"two\\\"\"")
      (lyric-eval-region (point-min) (point-max) t)
      (should (equal (buffer-string) "\"one\\n\\\"two\\\"\""))
      (erase-buffer)
      (insert "(let ((window '(:id 7))) `(focus_window ',window))")
      (let ((lyric-result-width 90)) (lyric-eval-last-sexp t))
      (should (equal (buffer-string) "(focus_window '(:id 7))"))
      (erase-buffer)
      (insert "'(unquote @name)")
      (lyric-eval-last-sexp t)
      (should (equal (buffer-string) ", @name"))
      (erase-buffer)
      (insert "(missing_function)")
      (should-error (lyric-eval-last-sexp t) :type 'user-error)
      (should (equal (buffer-string) "(missing_function)")))))

(ert-deftest lyric-defn-macroexpansion-with-test-runtime ()
  "Inspect plain defn and replace an explicit inspection with its expansion."
  (skip-unless (getenv "LYRIC_TEST_VRSCTL"))
  (let ((lyric-vrsctl-command (getenv "LYRIC_TEST_VRSCTL"))
        (lyric-result-width 90)
        (source "(defn echo (x) \"Echo\" x)")
        (expansion "(def echo (fn (x) \"Echo\" x))"))
    (with-temp-buffer
      (insert source)
      (lyric-mode)
      (goto-char (1- (point-max)))
      (dolist (repeat '(nil t))
        (lyric-macroexpand-last-sexp repeat)
        (with-current-buffer "*Lyric Result*"
          (should (equal (buffer-string) (concat expansion "\n"))))
        (should (equal (buffer-string) source)))
      (erase-buffer)
      (insert (format "(macroexpand_1 '%s)" source))
      (goto-char (point-max))
      (lyric-eval-last-sexp t)
      (should (equal (buffer-string) expansion)))))

(ert-deftest lyric-macroexpansion-and-quit-with-test-runtime ()
  "Inspect a real service macro, then cancel an accidental server loop."
  (skip-unless (getenv "LYRIC_TEST_VRSCTL"))
  (let ((lyric-vrsctl-command (getenv "LYRIC_TEST_VRSCTL"))
        timer cancelled)
    (with-temp-buffer
      (insert "(srv! :emacs_abort_probe :interface '())")
      (lyric-mode)
      (goto-char (1- (point-max)))
      (lyric-macroexpand-last-sexp nil)
      (with-current-buffer "*Lyric Result*"
        (should (string-match-p "register" (buffer-string)))
        (should (string-match-p "loop" (buffer-string))))
      (unwind-protect
          (progn
            ;; The same quit flag is set by C-g while waiting for output.
            (setq timer (run-at-time 0.1 nil (lambda () (setq quit-flag t))))
            (condition-case nil
                (lyric-eval-last-sexp t)
              (quit (setq cancelled t))))
        (when timer (cancel-timer timer)))
      (should cancelled)
      (should (equal (buffer-string) "(srv! :emacs_abort_probe :interface '())"))
      (erase-buffer)
      (insert "(+ 20 22)")
      (goto-char (point-max))
      (lyric-eval-last-sexp t)
      (should (equal (buffer-string) "42")))))

(ert-deftest lyric-mode-indents-data-and-calls ()
  (dolist (example
           '(("((:name :echo\n:node \"alpha\"\n:interface ((ping\nx))))"
              . "((:name :echo\n  :node \"alpha\"\n  :interface ((ping\n               x))))")
             ("'(alpha\n(beta\ngamma))" . "'(alpha\n  (beta\n   gamma))")
             ("(:echo\n(:name :echo\n:pid 1)\n:clock (:name :clock))"
              . "(:echo\n (:name :echo\n  :pid 1)\n :clock (:name :clock))")
             ("(pretty\n(ls_srv)\n40)" . "(pretty\n  (ls_srv)\n  40)")
             ("(pretty (ls_srv)\n40)" . "(pretty (ls_srv)\n        40)")
             ("(defn f (x)\n(begin\n(list x\n1)))" . "(defn f (x)\n  (begin\n    (list x\n          1)))")))
    (with-temp-buffer
      (insert (car example))
      (lyric-mode)
      (indent-region (point-min) (point-max))
      (should (equal (buffer-string) (cdr example)))
      (indent-region (point-min) (point-max))
      (should (equal (buffer-string) (cdr example))))))

(ert-deftest lyric-mode-indentation-preserves-raw-block-content ()
  (with-temp-buffer
    (insert lyric-test--block-expression)
    (lyric-mode)
    (indent-region (point-min) (point-max))
    (should (equal (buffer-string) lyric-test--block-expression))))

(ert-deftest lyric-mode-indent-sexp-uses-data-indentation ()
  (with-temp-buffer
    (insert "((:name :echo\n:node \"alpha\")\n(:name :clock))")
    (lyric-mode)
    (goto-char (point-min))
    (indent-sexp)
    (should (equal (buffer-string)
                   "((:name :echo\n  :node \"alpha\")\n (:name :clock))"))))

(ert-deftest lyric-evaluation-displays-pretty-text-and-preserves-source ()
  (with-temp-buffer
    (insert lyric-test--block-expression)
    (lyric-mode)
    (goto-char (point-max))
    (cl-letf (((symbol-function 'lyric--run-region)
               (lambda (start end command output &rest _)
                 (should (equal (buffer-substring-no-properties start end)
                                lyric-test--block-expression))
                 (should (string-match-p "--format pretty --width 90 --raw" command))
                 (with-current-buffer output (insert "((1 2)\n (3 4))\n"))
                 0))
              ((symbol-function 'display-buffer) #'ignore))
      (lyric-eval-last-sexp nil))
    (should (equal (buffer-string) lyric-test--block-expression))
    (with-current-buffer "*Lyric Result*"
      (should (eq major-mode 'lyric-mode))
      (should buffer-read-only)
      (should (equal (buffer-string) "((1 2)\n (3 4))\n")))))

(ert-deftest lyric-evaluation-replacement-indents-in-context ()
  (dolist (command '(lyric-eval-last-sexp lyric-eval-region))
    (with-temp-buffer
      (insert "(begin\n  (ls_srv))")
      (lyric-mode)
      (goto-char (- (point-max) 2))
      (cl-letf (((symbol-function 'lyric--run-region)
                 (lambda (_start _end command output &rest _)
                   (should (string-match-p "--format pretty --width 90" command))
                   (should-not (string-match-p "--raw" command))
                   (with-current-buffer output
                     (insert "((:name :echo\n  :node \"alpha\")\n (:name :clock))\n"))
                   0)))
        (if (eq command 'lyric-eval-last-sexp)
            (lyric-eval-last-sexp t)
          (lyric-eval-region 10 (1+ (point)) t)))
      (should (equal (buffer-string)
                     "(begin\n  ((:name :echo\n    :node \"alpha\")\n   (:name :clock)))")))))

(ert-deftest lyric-evaluation-does-not-replace-source-on-error ()
  (with-temp-buffer
    (insert "(missing)")
    (lyric-mode)
    (goto-char (point-max))
    (cl-letf (((symbol-function 'lyric--run-region)
               (lambda (_start _end _command output &rest _)
                 (with-current-buffer output (insert "not a result"))
                 1))
              ((symbol-function 'display-buffer) #'ignore))
      (should-error (lyric-eval-last-sexp t) :type 'user-error))
    (should (equal (buffer-string) "(missing)"))))

(ert-deftest lyric-eval-buffer-requests-commented-editor-output ()
  (with-temp-buffer
    (insert "(pretty (ls_srv))")
    (lyric-mode)
    (cl-letf (((symbol-function 'lyric--run-region)
               (lambda (_start _end command _output &rest _)
                 (should (string-match-p "--format editor --width 90 --raw" command))
                 0))
              ((symbol-function 'display-buffer) #'ignore))
      (lyric-eval-buffer t))))

(defconst lyric-test--block-expression
  (concat
   "(exec \"bash\" \"-s\"\n"
   "      :stdin \"\"\"\n"
   "      printf \"%s\\n\" \"$1\"\n"
   "      # Shell parens are not Lyric forms: (hello)\n"
   "      path='C:\\tmp'\n"
   "      \"\"\")"))

(ert-deftest lyric-mode-block-string-is-one-sexp ()
  (with-temp-buffer
    (insert lyric-test--block-expression)
    (lyric-mode)
    (syntax-propertize (point-max))
    (goto-char (point-max))
    (should (equal (lyric--last-sexp-source)
                   lyric-test--block-expression))))

(ert-deftest lyric-mode-block-content-has-string-syntax ()
  (with-temp-buffer
    (insert lyric-test--block-expression)
    (lyric-mode)
    (syntax-propertize (point-max))
    (goto-char (point-min))
    (search-forward "(hello)")
    (should (nth 3 (syntax-ppss)))))

(ert-deftest lyric-mode-last-sexp-preserves-multiline-source ()
  (with-temp-buffer
    (insert lyric-test--block-expression "\n\n")
    (lyric-mode)
    (syntax-propertize (point-max))
    (goto-char (point-max))
    (should (equal (lyric--last-sexp-source)
                   lyric-test--block-expression))))

(ert-deftest lyric-mode-last-sexp-includes-quote-prefix ()
  (dolist (source '("'((1 2) (3 4))" "''(a b)" "'name"))
    (with-temp-buffer
      (insert source)
      (lyric-mode)
      (should (equal (lyric--last-sexp-source) source)))))

(ert-deftest lyric-mode-preserves-all-reader-prefixes-and-gaps ()
  (dolist (source '("`(a ,x ,@xs)" ",(compute x)" ",@(get record :commands)"
                    "',name" "',@xs" "``(a ,,x)" ", @name" ",@name"
                    "` # template\n (a , # hole\n x)" "' # literal\n name"))
    (with-temp-buffer
      (insert "previous\n" source)
      (lyric-mode)
      (goto-char (point-max))
      (should (equal (lyric--last-sexp-source) source)))))

(ert-deftest lyric-mode-comma-at-is-contextual ()
  (with-temp-buffer
    (insert ",@name , @name \"literal ,@name\" # comment ,@name\n\"\"\"raw ,@name\"\"\"")
    (lyric-mode)
    (goto-char (point-min))
    (search-forward "@")
    (should (eq (syntax-class (syntax-after (1- (point)))) 6)) ; prefix
    (search-forward "@")
    (should (eq (syntax-class (syntax-after (1- (point)))) 3)) ; symbol
    (search-forward "@")
    (should (nth 3 (syntax-ppss)))
    (should-not (get-text-property (1- (point)) 'syntax-table))
    (search-forward "@")
    (should (nth 4 (syntax-ppss)))
    (should-not (get-text-property (1- (point)) 'syntax-table))
    (search-forward "@")
    (should (nth 3 (syntax-ppss)))
    (should-not (get-text-property (1- (point)) 'syntax-table))))

(ert-deftest lyric-mode-quotation-context-is-depth-aware ()
  (dolist (example '(("`(outer `(inner ,(later) ,,(now)))" "later" t)
                     ("`(outer `(inner ,(later) ,,(now)))" "now" nil)
                     ("`(outer ',(now))" "now" nil)
                     ("'`(outer ,(now))" "now" t)
                     ("(quasiquote (outer (unquote (now))))" "now" nil)
                     ("(quote (quasiquote (outer (unquote (now)))))" "now" t)))
    (with-temp-buffer
      (insert (nth 0 example))
      (lyric-mode)
      (goto-char (point-min))
      (search-forward (concat "(" (nth 1 example)))
      (goto-char (- (point) (1+ (length (nth 1 example)))))
      (should (eq (not (null (lyric--data-list-p (point)))) (nth 2 example))))))

(ert-deftest lyric-mode-indents-template-holes-as-code ()
  (with-temp-buffer
    (insert "`(a\n,(compute\nfirst\nsecond)\n(nested\nvalues))")
    (lyric-mode)
    (indent-region (point-min) (point-max))
    (should (equal (buffer-string)
                   "`(a\n  ,(compute\n     first\n     second)\n  (nested\n   values))"))
    (let ((once (buffer-string)))
      (indent-region (point-min) (point-max))
      (should (equal (buffer-string) once)))))

(ert-deftest lyric-mode-repropertizes-edited-comma-at ()
  (with-temp-buffer
    (insert ",@name")
    (lyric-mode)
    (goto-char 2)
    (insert " ")
    (syntax-propertize (point-max))
    (should (eq (syntax-class (syntax-after 3)) 3))
    (goto-char (point-max))
    (should (equal (lyric--last-sexp-source) ", @name"))))

(ert-deftest lyric-mode-inline-block-may-end-with-a-quote ()
  (let ((source "(list \"\"\"inline \"quoted\"\"\"\")"))
    (with-temp-buffer
      (insert source)
      (lyric-mode)
      (syntax-propertize (point-max))
      (goto-char (point-max))
      (should (equal (lyric--last-sexp-source) source)))))

(ert-deftest lyric-macroexpand-wraps-exact-source-without-evaluating-it ()
  (dolist (repeat '(nil t))
    (with-temp-buffer
      (let ((source "(when! true\n  (notify \"literal ` ,@x\"))")
            captured)
        (insert source)
        (lyric-mode)
        (goto-char (point-max))
        (cl-letf (((symbol-function 'lyric--eval)
                   (lambda (start end replace &optional _editor source-result)
                     (should-not replace)
                     (should source-result)
                     (setq captured (buffer-substring-no-properties start end)))))
          (lyric-macroexpand-last-sexp repeat))
        (should (equal captured (format "(%s (quote %s))"
                                        (if repeat "macroexpand" "macroexpand_1") source)))
        (should (equal (buffer-string) source))))))

(ert-deftest lyric-macroexpand-preserves-string-literal-syntax ()
  (with-temp-buffer
    (insert "\"literal\"")
    (lyric-mode)
    (goto-char (point-max))
    (cl-letf (((symbol-function 'lyric--run-region)
               (lambda (start end command output &rest _)
                 (should (equal (buffer-substring-no-properties start end)
                                "(macroexpand_1 (quote \"literal\"))"))
                 (should-not (string-match-p "--raw" command))
                 (with-current-buffer output (insert "\"literal\"\n"))
                 0))
              ((symbol-function 'display-buffer) #'ignore))
      (lyric-macroexpand-last-sexp nil))
    (should (equal (buffer-string) "\"literal\""))
    (with-current-buffer "*Lyric Result*"
      (should (equal (buffer-string) "\"literal\"\n")))))

(ert-deftest lyric-indents-macro-definitions-and-marked-call-bodies ()
  (with-temp-buffer
    (insert "(defmacro when (test & body)\n`(if ,test (begin ,@body) nil))\n(when! true\n(notify \"hello\"))")
    (lyric-mode)
    (indent-region (point-min) (point-max))
    (should (equal (buffer-string)
                   "(defmacro when (test & body)\n  `(if ,test (begin ,@body) nil))\n(when! true\n  (notify \"hello\"))"))))

(provide 'lyric-mode-tests)

;;; lyric-mode-tests.el ends here
