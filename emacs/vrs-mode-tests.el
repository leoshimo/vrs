;;; vrs-mode-tests.el --- Tests for vrs-mode -*- lexical-binding: t; -*-

(require 'ert)
(require 'vrs-mode)
(require 'cl-lib)

(defconst vrs-test--block-expression
  (concat
   "(exec \"bash\" \"-s\"\n"
   "      :stdin \"\"\"\n"
   "      printf \"%s\\n\" \"$1\"\n"
   "      # Shell parens are not Lyric forms: (hello)\n"
   "      path='C:\\tmp'\n"
   "      \"\"\")"))

(ert-deftest vrs-evaluation-selects-the-form-at-a-closing-paren ()
  (dolist (source '("(ls_srv)" "'((1 2) (3 4))" "`(a ,(get x 0))"
                    "(srv! :test :interface '())"))
    (with-temp-buffer
      (insert source)
      (vrs-mode)
      (dolist (position (list (1- (point-max)) (point-max)))
        (goto-char position)
        (should (equal (vrs--last-sexp-source) source)))))
  (with-temp-buffer
    (insert "(begin (ls_srv))")
    (vrs-mode)
    (goto-char (- (point-max) 2))
    (should (equal (vrs--last-sexp-source) "(ls_srv)"))
    (forward-char)
    (should (equal (vrs--last-sexp-source) "(begin (ls_srv))"))))

(ert-deftest vrs-evaluation-process-keeps-stdout-and-stderr-separate ()
  (with-temp-buffer
    (insert "exact source: \"東京\"\n")
    (let ((output (generate-new-buffer " *VRS test output*"))
          (errors (generate-new-buffer " *VRS test errors*")))
      (unwind-protect
          (progn
            (should (= 7 (vrs--run-region
                          (point-min) (point-max)
                          "sh -c 'cat; printf diagnostic >&2; exit 7'" output errors)))
            (should (equal (with-current-buffer output (buffer-string))
                           "exact source: \"東京\"\n"))
            (should (equal (with-current-buffer errors (buffer-string)) "diagnostic")))
        (kill-buffer output)
        (kill-buffer errors)))))

(ert-deftest vrs-evaluation-quit-kills-the-client-and-preserves-source ()
  (with-temp-buffer
    (insert "(srv! :test :interface '())")
    (vrs-mode)
    (goto-char (1- (point-max)))
    (let ((vrs-vrsctl-command "sleep 30 #")
          (make-real-process (symbol-function 'make-process))
          child cancelled)
      (cl-letf (((symbol-function 'make-process)
                 (lambda (&rest args)
                   (setq child (apply make-real-process args))))
                ((symbol-function 'accept-process-output)
                 (lambda (&rest _) (signal 'quit nil))))
        (condition-case nil
            (vrs-eval-last-sexp t)
          (quit (setq cancelled t))))
      (should cancelled)
      (should child)
      (should-not (process-live-p child))
      (should (equal (buffer-string) "(srv! :test :interface '())")))))

(ert-deftest vrs-evaluation-with-test-runtime ()
  "Optional end-to-end evaluation, enabled by the terminal test harness."
  (skip-unless (getenv "VRS_TEST_VRSCTL"))
  (let ((vrs-vrsctl-command (getenv "VRS_TEST_VRSCTL"))
        (vrs-result-width 10))
    (with-temp-buffer
      (insert "(pretty '((1 2) (3 4)) 10)")
      (vrs-mode)
      (goto-char (point-max))
      (vrs-eval-last-sexp nil)
      (with-current-buffer "*VRS Result*"
        (should (equal (buffer-string) "((1 2)\n (3 4))\n")))
      (erase-buffer)
      (insert "(list? (ls_srv))")
      (goto-char (1- (point-max)))
      (vrs-eval-last-sexp t)
      (should (equal (buffer-string) "true"))
      (erase-buffer)
      (insert "'((1 2) (3 4))")
      (vrs-eval-last-sexp t)
      (should (equal (buffer-string) "((1 2)\n (3 4))"))
      (erase-buffer)
      (insert "\"one\\n\\\"two\\\"\"")
      (vrs-eval-region (point-min) (point-max) t)
      (should (equal (buffer-string) "\"one\\n\\\"two\\\"\""))
      (erase-buffer)
      (insert "(let ((window '(:id 7))) `(focus_window ',window))")
      (let ((vrs-result-width 90)) (vrs-eval-last-sexp t))
      (should (equal (buffer-string) "(focus_window '(:id 7))"))
      (erase-buffer)
      (insert "'(unquote @name)")
      (vrs-eval-last-sexp t)
      (should (equal (buffer-string) ", @name"))
      (erase-buffer)
      (insert "(missing_function)")
      (should-error (vrs-eval-last-sexp t) :type 'user-error)
      (should (equal (buffer-string) "(missing_function)")))))

(ert-deftest vrs-defn-macroexpansion-with-test-runtime ()
  "Inspect defn! and replace an explicit inspection with its expansion."
  (skip-unless (getenv "VRS_TEST_VRSCTL"))
  (let ((vrs-vrsctl-command (getenv "VRS_TEST_VRSCTL"))
        (vrs-result-width 90)
        (source "(defn! echo (x) \"Echo\" x)")
        (expansion "(def echo (fn (x) \"Echo\" x))"))
    (with-temp-buffer
      (insert source)
      (vrs-mode)
      (goto-char (1- (point-max)))
      (dolist (repeat '(nil t))
        (vrs-macroexpand-last-sexp repeat)
        (with-current-buffer "*VRS Result*"
          (should (equal (buffer-string) (concat expansion "\n"))))
        (should (equal (buffer-string) source)))
      (erase-buffer)
      (insert (format "(macroexpand_1 '%s)" source))
      (goto-char (point-max))
      (vrs-eval-last-sexp t)
      (should (equal (buffer-string) expansion)))))

(ert-deftest vrs-macroexpansion-and-quit-with-test-runtime ()
  "Inspect a real service macro, then cancel an accidental server loop."
  (skip-unless (getenv "VRS_TEST_VRSCTL"))
  (let ((vrs-vrsctl-command (getenv "VRS_TEST_VRSCTL"))
        timer cancelled)
    (with-temp-buffer
      (insert "(srv! :emacs_abort_probe :interface '())")
      (vrs-mode)
      (goto-char (1- (point-max)))
      (vrs-macroexpand-last-sexp nil)
      (with-current-buffer "*VRS Result*"
        (should (string-match-p "register" (buffer-string)))
        (should (string-match-p "loop" (buffer-string))))
      (unwind-protect
          (progn
            ;; The same quit flag is set by C-g while waiting for output.
            (setq timer (run-at-time 0.1 nil (lambda () (setq quit-flag t))))
            (condition-case nil
                (vrs-eval-last-sexp t)
              (quit (setq cancelled t))))
        (when timer (cancel-timer timer)))
      (should cancelled)
      (should (equal (buffer-string) "(srv! :emacs_abort_probe :interface '())"))
      (erase-buffer)
      (insert "(+ 20 22)")
      (goto-char (point-max))
      (vrs-eval-last-sexp t)
      (should (equal (buffer-string) "42")))))

(ert-deftest vrs-mode-indents-data-and-calls ()
  (dolist (example
           '(("((:name :echo\n:node \"alpha\"\n:interface ((ping\nx))))"
              . "((:name :echo\n  :node \"alpha\"\n  :interface ((ping\n               x))))")
             ("'(alpha\n(beta\ngamma))" . "'(alpha\n  (beta\n   gamma))")
             ("(:echo\n(:name :echo\n:pid 1)\n:clock (:name :clock))"
              . "(:echo\n (:name :echo\n  :pid 1)\n :clock (:name :clock))")
             ("(pretty\n(ls_srv)\n40)" . "(pretty\n  (ls_srv)\n  40)")
             ("(pretty (ls_srv)\n40)" . "(pretty (ls_srv)\n        40)")
             ("(defn! f (x)\n(begin\n(list x\n1)))" . "(defn! f (x)\n  (begin\n    (list x\n          1)))")))
    (with-temp-buffer
      (insert (car example))
      (vrs-mode)
      (indent-region (point-min) (point-max))
      (should (equal (buffer-string) (cdr example)))
      (indent-region (point-min) (point-max))
      (should (equal (buffer-string) (cdr example))))))

(ert-deftest vrs-mode-indentation-preserves-raw-block-content ()
  (with-temp-buffer
    (insert vrs-test--block-expression)
    (vrs-mode)
    (indent-region (point-min) (point-max))
    (should (equal (buffer-string) vrs-test--block-expression))))

(ert-deftest vrs-mode-indent-sexp-uses-data-indentation ()
  (with-temp-buffer
    (insert "((:name :echo\n:node \"alpha\")\n(:name :clock))")
    (vrs-mode)
    (goto-char (point-min))
    (indent-sexp)
    (should (equal (buffer-string)
                   "((:name :echo\n  :node \"alpha\")\n (:name :clock))"))))

(ert-deftest vrs-evaluation-displays-pretty-text-and-preserves-source ()
  (with-temp-buffer
    (insert vrs-test--block-expression)
    (vrs-mode)
    (goto-char (point-max))
    (cl-letf (((symbol-function 'vrs--run-region)
               (lambda (start end command output &rest _)
                 (should (equal (buffer-substring-no-properties start end)
                                vrs-test--block-expression))
                 (should (string-match-p "--format pretty --width 90 --raw" command))
                 (with-current-buffer output (insert "((1 2)\n (3 4))\n"))
                 0))
              ((symbol-function 'display-buffer) #'ignore))
      (vrs-eval-last-sexp nil))
    (should (equal (buffer-string) vrs-test--block-expression))
    (with-current-buffer "*VRS Result*"
      (should (eq major-mode 'vrs-mode))
      (should buffer-read-only)
      (should (equal (buffer-string) "((1 2)\n (3 4))\n")))))

(ert-deftest vrs-evaluation-replacement-indents-in-context ()
  (dolist (command '(vrs-eval-last-sexp vrs-eval-region))
    (with-temp-buffer
      (insert "(begin\n  (ls_srv))")
      (vrs-mode)
      (goto-char (- (point-max) 2))
      (cl-letf (((symbol-function 'vrs--run-region)
                 (lambda (_start _end command output &rest _)
                   (should (string-match-p "--format pretty --width 90" command))
                   (should-not (string-match-p "--raw" command))
                   (with-current-buffer output
                     (insert "((:name :echo\n  :node \"alpha\")\n (:name :clock))\n"))
                   0)))
        (if (eq command 'vrs-eval-last-sexp)
            (vrs-eval-last-sexp t)
          (vrs-eval-region 10 (1+ (point)) t)))
      (should (equal (buffer-string)
                     "(begin\n  ((:name :echo\n    :node \"alpha\")\n   (:name :clock)))")))))

(ert-deftest vrs-evaluation-does-not-replace-source-on-error ()
  (with-temp-buffer
    (insert "(missing)")
    (vrs-mode)
    (goto-char (point-max))
    (cl-letf (((symbol-function 'vrs--run-region)
               (lambda (_start _end _command output &rest _)
                 (with-current-buffer output (insert "not a result"))
                 1))
              ((symbol-function 'display-buffer) #'ignore))
      (should-error (vrs-eval-last-sexp t) :type 'user-error))
    (should (equal (buffer-string) "(missing)"))))

(ert-deftest vrs-eval-buffer-requests-commented-editor-output ()
  (with-temp-buffer
    (insert "(pretty (ls_srv))")
    (vrs-mode)
    (cl-letf (((symbol-function 'vrs--run-region)
               (lambda (_start _end command _output &rest _)
                 (should (string-match-p "--format editor --width 90 --raw" command))
                 0))
              ((symbol-function 'display-buffer) #'ignore))
      (vrs-eval-buffer t))))


(ert-deftest vrs-mode-block-string-is-one-sexp ()
  (with-temp-buffer
    (insert vrs-test--block-expression)
    (vrs-mode)
    (syntax-propertize (point-max))
    (goto-char (point-max))
    (should (equal (vrs--last-sexp-source)
                   vrs-test--block-expression))))

(ert-deftest vrs-mode-block-content-has-string-syntax ()
  (with-temp-buffer
    (insert vrs-test--block-expression)
    (vrs-mode)
    (syntax-propertize (point-max))
    (goto-char (point-min))
    (search-forward "(hello)")
    (should (nth 3 (syntax-ppss)))))

(ert-deftest vrs-mode-last-sexp-preserves-multiline-source ()
  (with-temp-buffer
    (insert vrs-test--block-expression "\n\n")
    (vrs-mode)
    (syntax-propertize (point-max))
    (goto-char (point-max))
    (should (equal (vrs--last-sexp-source)
                   vrs-test--block-expression))))

(ert-deftest vrs-mode-last-sexp-includes-quote-prefix ()
  (dolist (source '("'((1 2) (3 4))" "''(a b)" "'name"))
    (with-temp-buffer
      (insert source)
      (vrs-mode)
      (should (equal (vrs--last-sexp-source) source)))))

(ert-deftest vrs-mode-preserves-all-reader-prefixes-and-gaps ()
  (dolist (source '("`(a ,x ,@xs)" ",(compute x)" ",@(get record :commands)"
                    "',name" "',@xs" "``(a ,,x)" ", @name" ",@name"
                    "` # template\n (a , # hole\n x)" "' # literal\n name"))
    (with-temp-buffer
      (insert "previous\n" source)
      (vrs-mode)
      (goto-char (point-max))
      (should (equal (vrs--last-sexp-source) source)))))

(ert-deftest vrs-mode-comma-at-is-contextual ()
  (with-temp-buffer
    (insert ",@name , @name \"literal ,@name\" # comment ,@name\n\"\"\"raw ,@name\"\"\"")
    (vrs-mode)
    (goto-char (point-min))
    (search-forward "@")
    (should (eq (syntax-class (syntax-after (1- (point)))) 6)) ; prefix
    (search-forward "@")
    (should (eq (syntax-class (syntax-after (1- (point)))) 3)) ; symbol
    (search-forward "@")
    (should (nth 3 (syntax-ppss)))
    (should-not (eq (syntax-class (syntax-after (1- (point)))) 6))
    (search-forward "@")
    (should (nth 4 (syntax-ppss)))
    (should-not (eq (syntax-class (syntax-after (1- (point)))) 6))
    (search-forward "@")
    (should (nth 3 (syntax-ppss)))
    (should-not (eq (syntax-class (syntax-after (1- (point)))) 6))))

(ert-deftest vrs-mode-quotation-context-is-depth-aware ()
  (dolist (example '(("`(outer `(inner ,(later) ,,(now)))" "later" t)
                     ("`(outer `(inner ,(later) ,,(now)))" "now" nil)
                     ("`(outer ',(now))" "now" nil)
                     ("'`(outer ,(now))" "now" t)
                     ("(quasiquote (outer (unquote (now))))" "now" nil)
                     ("(quote (quasiquote (outer (unquote (now)))))" "now" t)))
    (with-temp-buffer
      (insert (nth 0 example))
      (vrs-mode)
      (goto-char (point-min))
      (search-forward (concat "(" (nth 1 example)))
      (goto-char (- (point) (1+ (length (nth 1 example)))))
      (should (eq (not (null (vrs--data-list-p (point)))) (nth 2 example))))))

(ert-deftest vrs-mode-indents-template-holes-as-code ()
  (with-temp-buffer
    (insert "`(a\n,(compute\nfirst\nsecond)\n(nested\nvalues))")
    (vrs-mode)
    (indent-region (point-min) (point-max))
    (should (equal (buffer-string)
                   "`(a\n  ,(compute\n     first\n     second)\n  (nested\n   values))"))
    (let ((once (buffer-string)))
      (indent-region (point-min) (point-max))
      (should (equal (buffer-string) once)))))

(ert-deftest vrs-mode-repropertizes-edited-comma-at ()
  (with-temp-buffer
    (insert ",@name")
    (vrs-mode)
    (goto-char 2)
    (insert " ")
    (syntax-propertize (point-max))
    (should (eq (syntax-class (syntax-after 3)) 3))
    (goto-char (point-max))
    (should (equal (vrs--last-sexp-source) ", @name"))))

(ert-deftest vrs-mode-inline-block-may-end-with-a-quote ()
  (let ((source "(list \"\"\"inline \"quoted\"\"\"\")"))
    (with-temp-buffer
      (insert source)
      (vrs-mode)
      (syntax-propertize (point-max))
      (goto-char (point-max))
      (should (equal (vrs--last-sexp-source) source)))))

(ert-deftest vrs-macroexpand-wraps-exact-source-without-evaluating-it ()
  (dolist (repeat '(nil t))
    (with-temp-buffer
      (let ((source "(when! true\n  (notify \"literal ` ,@x\"))")
            captured)
        (insert source)
        (vrs-mode)
        (goto-char (point-max))
        (cl-letf (((symbol-function 'vrs--eval)
                   (lambda (start end replace &optional _editor source-result)
                     (should-not replace)
                     (should source-result)
                     (setq captured (buffer-substring-no-properties start end)))))
          (vrs-macroexpand-last-sexp repeat))
        (should (equal captured (format "(%s (quote %s))"
                                        (if repeat "macroexpand" "macroexpand_1") source)))
        (should (equal (buffer-string) source))))))

(ert-deftest vrs-macroexpand-preserves-string-literal-syntax ()
  (with-temp-buffer
    (insert "\"literal\"")
    (vrs-mode)
    (goto-char (point-max))
    (cl-letf (((symbol-function 'vrs--run-region)
               (lambda (start end command output &rest _)
                 (should (equal (buffer-substring-no-properties start end)
                                "(macroexpand_1 (quote \"literal\"))"))
                 (should-not (string-match-p "--raw" command))
                 (with-current-buffer output (insert "\"literal\"\n"))
                 0))
              ((symbol-function 'display-buffer) #'ignore))
      (vrs-macroexpand-last-sexp nil))
    (should (equal (buffer-string) "\"literal\""))
    (with-current-buffer "*VRS Result*"
      (should (equal (buffer-string) "\"literal\"\n")))))

(ert-deftest vrs-indents-macro-definitions-and-marked-call-bodies ()
  (with-temp-buffer
    (insert "(defmacro when (test & body)\n`(if ,test (begin ,@body) nil))\n(when! true\n(notify \"hello\"))")
    (vrs-mode)
    (indent-region (point-min) (point-max))
    (should (equal (buffer-string)
                   "(defmacro when (test & body)\n  `(if ,test (begin ,@body) nil))\n(when! true\n  (notify \"hello\"))"))))

(ert-deftest vrs-mode-symbols-follow-the-lyric-reader ()
  (dolist (source '("service/name!?" "@name" "name#suffix" "name\"suffix"
                    "path\\part" "[]{};" "東京"))
    (with-temp-buffer
      (insert source)
      (vrs-mode)
      (goto-char (point-min))
      (forward-sexp)
      (should (= (point) (point-max)))
      (should (equal (vrs--last-sexp-source) source)))))

(ert-deftest vrs-mode-strings-are-complete-sexps ()
  (dolist (source '("\"escaped \\\"quote\\\"\"" "\"\"\"\"\"\"" "\"\"\"\"\"\"\""
                    "\"\"\"trailing\\\"\"\"" "\"\"\"inline \"quoted\"\"\"\""
                    "\"\"\"\n(raw \\ # \" ,@name)\n\"\"\""))
    (with-temp-buffer
      (insert source " (after)")
      (vrs-mode)
      (goto-char (point-min))
      (forward-sexp)
      (should (= (point) (1+ (length source))))
      (should (equal (vrs--last-sexp-source) source))
      (forward-sexp)
      (should (equal (vrs--last-sexp-source) "(after)")))))

(ert-deftest vrs-mode-highlights-lyric-syntax ()
  (with-temp-buffer
    (insert "(def answer -42)\n(defn! greet (name)\n  (if true :yes nil))\n"
            "# comment (def ignored)\n\"literal (def ignored)\"\n"
            "\"\"\"raw (def ignored)\"\"\"\n(var janet_only 1)")
    (vrs-mode)
    (font-lock-ensure)
    (dolist (example '(("def" . font-lock-keyword-face)
                       ("answer" . font-lock-variable-name-face)
                       ("-42" . font-lock-constant-face)
                       ("defn!" . font-lock-keyword-face)
                       ("greet" . font-lock-function-name-face)
                       ("if" . font-lock-keyword-face)
                       ("true" . font-lock-constant-face)
                       (":yes" . font-lock-constant-face)
                       ("nil" . font-lock-constant-face)
                       ("comment" . font-lock-comment-face)
                       ("literal" . font-lock-string-face)
                       ("raw" . font-lock-string-face)
                       ("var" . nil)))
      (goto-char (point-min))
      (search-forward (car example))
      (should (eq (get-text-property (1- (point)) 'face) (cdr example))))))

(ert-deftest vrs-mode-comments-and-defun-navigation ()
  (with-temp-buffer
    (insert "(defn! first ()\n  \"\"\"\n(inside raw string)\n\"\"\")\n\n(defn! second () 2)")
    (vrs-mode)
    (goto-char (point-max))
    (beginning-of-defun)
    (should (looking-at "(defn! second"))
    (beginning-of-defun)
    (should (= (point) (point-min)))
    (end-of-defun)
    (should (looking-at "\n(defn! second"))
    (comment-region (point-min) (point-max))
    (goto-char (point-min))
    (search-forward "first")
    (should (nth 4 (syntax-ppss)))
    (uncomment-region (point-min) (point-max))
    (goto-char (point-min))
    (should (looking-at "(defn! first"))))

(ert-deftest vrs-mode-reparses-an-edited-raw-delimiter ()
  (with-temp-buffer
    (insert "\"\"\"raw\"\"\"\n(def answer 42)")
    (vrs-mode)
    (goto-char (point-min))
    (search-forward "raw\"")
    (delete-char -1)
    (font-lock-ensure)
    (search-forward "answer")
    (should (nth 3 (syntax-ppss)))
    (goto-char (point-min))
    (search-forward "raw")
    (insert "\"")
    (font-lock-ensure)
    (search-forward "answer")
    (should-not (nth 3 (syntax-ppss)))
    (should (eq (get-text-property (1- (point)) 'face) 'font-lock-variable-name-face))))

(provide 'vrs-mode-tests)

;;; vrs-mode-tests.el ends here
