;;; lyric-mode.el --- Major mode and evaluation helpers for Lyric -*- lexical-binding: t; -*-

(require 'janet-mode)
(require 'subr-x)

(defgroup lyric nil
  "Editing Lyric programs."
  :group 'languages)

(defcustom lyric-vrsctl-command "vrsctl"
  "Base vrsctl command used by Lyric evaluation commands."
  :type 'string
  :group 'lyric)

(defcustom lyric-result-width 90
  "Target column width for evaluated values."
  :type '(integer :tag "Columns")
  :group 'lyric)

(defun lyric--prefixes-before (position)
  "Return (START . PREFIXES) for reader prefixes before POSITION.
PREFIXES are ordered from outermost to innermost; preserve spaces/comments."
  (save-excursion
    (goto-char position)
    (let ((start position) prefixes done)
      (while (not done)
        (forward-comment (- (point-max)))
        (cond
         ((and (eq (char-before) ?@)
               (eq (char-before (1- (point))) ?,))
          (backward-char 2) (push ",@" prefixes) (setq start (point)))
         ((memq (char-before) '(?\' ?` ?,))
          (push (char-to-string (char-before)) prefixes)
          (backward-char) (setq start (point)))
         (t (setq done t))))
      (cons start prefixes))))

(defun lyric--prefix-context (context prefix)
  "Apply PREFIX to CONTEXT, a (LITERAL DEPTH DATA) list."
  (pcase-let ((`(,literal ,depth ,data) context))
    (cond
     (literal context)
     ((and (equal prefix "'") (= depth 0)) (list t 0 t))
     ((equal prefix "`") (list nil (1+ depth) t))
     ((and (member prefix '("," ",@")) (> depth 0))
      (list nil (1- depth) (> depth 1)))
     (t (list literal depth data)))))

(defun lyric--list-context (open)
  "Return the quotation and data context of the list at OPEN."
  (save-excursion
    (goto-char open)
    (let* ((parent (nth 1 (syntax-ppss)))
           (context (if parent (lyric--list-context parent) (list nil 0 nil))))
      ;; Long reader forms have the same indentation boundaries as prefixes.
      (when parent
        (goto-char (1+ parent))
        (forward-comment (point-max))
        (when (looking-at "\\(quote\\|quasiquote\\|unquote-splicing\\|unquote\\)\\_>")
          (setq context
                (lyric--prefix-context
                 context (cdr (assoc (match-string-no-properties 1)
                                     '(("quote" . "'") ("quasiquote" . "`")
                                       ("unquote" . ",") ("unquote-splicing" . ",@"))))))))
      (dolist (prefix (cdr (lyric--prefixes-before open)))
        (setq context (lyric--prefix-context context prefix)))
      (goto-char (1+ open))
      (forward-comment (point-max))
      (setf (nth 2 context)
            (or (nth 0 context) (> (nth 1 context) 0) (nth 2 context)
                (memq (char-after) '(?: ?\( ?\" ?\' ?\)))
                (looking-at "\\(?:-?[0-9]+\\|nil\\|true\\|false\\)\\_>")))
      context)))

(defun lyric--data-list-p (open)
  "Whether the list at OPEN contains data rather than a function call."
  (nth 2 (lyric--list-context open)))

(defun lyric--indent-column ()
  "Compute Lyric indentation without treating keywords as function names."
  (save-excursion
    (back-to-indentation)
    (let* ((state (syntax-ppss))
           (open (nth 1 state)))
      (cond
       ((nth 3 state) nil) ; Never change embedded/raw string content.
       ((not open) 0)
       ((eq (char-after) ?\))
        (goto-char open) (current-column))
       ((lyric--data-list-p open)
        (goto-char open) (1+ (current-column)))
       (t
        (goto-char open)
        (let ((body (+ (current-column) 2)))
          (forward-char)
          (let ((head (buffer-substring-no-properties
                       (point) (progn (forward-sexp) (point)))))
            (skip-chars-forward " \t")
            (if (or (eolp) (looking-at "#")
                    (member head '("begin" "defmacro" "for_syntax" "lambda" "loop"))
                    (string-suffix-p "!" head))
                body
              (current-column)))))))))

(defun lyric-indent-line ()
  "Indent a Lyric line, preserving raw string contents."
  (interactive)
  (let ((column (lyric--indent-column))
        (offset (- (point-max) (point))))
    (when column
      (indent-line-to column)
      (when (> (- (point-max) offset) (point))
        (goto-char (- (point-max) offset))))))

(defun lyric-indent-function (indent-point _state)
  "Use Lyric indentation at INDENT-POINT for `indent-sexp' too."
  (save-excursion
    (goto-char indent-point)
    (lyric--indent-column)))

(defvar lyric-mode-syntax-table
  (let ((table (copy-syntax-table janet-mode-syntax-table)))
    ;; Reader prefixes are part of a Lyric expression, never string delimiters.
    (modify-syntax-entry ?` "'" table)
    (modify-syntax-entry ?, "'" table)
    (modify-syntax-entry ?\' "'" table)
    table)
  "Syntax table used in `lyric-mode'.")

(defun lyric--syntax-propertize (_start _end)
  "Apply Lyric block-string syntax properties to the current buffer.

The Lyric reader treats the first three quotes in an opening quote run and the
last three quotes in a closing run as delimiters. Matching that behavior lets a
raw block end with a literal quote without confusing Emacs sexp navigation."
  (save-excursion
    (with-silent-modifications
      (remove-text-properties (point-min) (point-max)
                              '(syntax-table nil syntax-multiline nil))
      (syntax-ppss-flush-cache (point-min))
      (goto-char (point-min))
      (let ((inside-block nil))
        (while (re-search-forward "\"\{3,\}" nil t)
          (let* ((run-start (match-beginning 0))
                 (run-end (match-end 0))
                 (state (syntax-ppss run-start))
                 (delimiter-start
                  (cond
                   (inside-block (- run-end 3))
                   ((or (nth 3 state) (nth 4 state)) nil)
                   (t run-start))))
            (when delimiter-start
              (put-text-property delimiter-start (1+ delimiter-start)
                                 'syntax-table (string-to-syntax "|"))
              (put-text-property (1+ delimiter-start) (+ delimiter-start 3)
                                 'syntax-table (string-to-syntax "."))
              (put-text-property run-start run-end 'syntax-multiline t)
              (setq inside-block (not inside-block))
              (syntax-ppss-flush-cache run-start)))))
      ;; Only the @ immediately following comma is a reader prefix. Elsewhere
      ;; @ belongs to symbols: , @name and ,@name are different expressions.
      (goto-char (point-min))
      (while (re-search-forward ",@" nil t)
        (let* ((start (match-beginning 0))
               (state (save-excursion (syntax-ppss start))))
          (unless (or (nth 3 state) (nth 4 state))
            (put-text-property (1+ start) (+ start 2)
                               'syntax-table (string-to-syntax "'"))
            (syntax-ppss-flush-cache start)))))))

(defun lyric--last-sexp-bounds ()
  "Return the bounds of the Lyric expression at a closing paren or before point.

Unlike `pp-last-sexp', this preserves the exact source text, including raw
block strings, rather than reading and printing it as Emacs Lisp."
  (save-excursion
    (when (and (eq (char-after) ?\))
               (not (nth 3 (syntax-ppss)))
               (not (nth 4 (syntax-ppss))))
      (forward-char))
    (skip-chars-backward " \t\r\n")
    (let ((end (point)))
      (backward-sexp)
      (goto-char (car (lyric--prefixes-before (point))))
      (cons (point) end))))

(defun lyric--run-region (start end command output errors)
  "Send START to END to COMMAND, collecting OUTPUT and ERRORS.
Wait interruptibly; quitting terminates the client, including when it is
waiting for a service loop.  The shell execs the client so it cannot be
orphaned when Emacs deletes the process."
  (let ((inhibit-quit nil)
        (stderr (make-pipe-process :name "Lyric errors" :buffer errors
                                   :noquery t :sentinel #'ignore))
        process)
    (unwind-protect
        (progn
          (setq process
                (make-process :name "Lyric evaluation" :buffer output
                              :command (list shell-file-name shell-command-switch
                                             (concat "exec " command))
                              :connection-type 'pipe :coding 'utf-8-unix
                              :stderr stderr :noquery t :sentinel #'ignore))
          (process-send-region process start end)
          (process-send-eof process)
          (while (process-live-p process)
            (accept-process-output process 0.05))
          ;; Drain final output from both pipes before inspecting the status.
          (while (accept-process-output process 0.01))
          (while (accept-process-output stderr 0.01))
          (process-exit-status process))
      (dolist (child (list process stderr))
        (when (and child (process-live-p child))
          (delete-process child))))))

(defun lyric--last-sexp-source ()
  "Return the exact Lyric expression preceding point."
  (pcase-let ((`(,start . ,end) (lyric--last-sexp-bounds)))
    (buffer-substring-no-properties start end)))

(defun lyric--eval (start end replace &optional editor-format source-result)
  "Evaluate START to END; optionally REPLACE or request EDITOR-FORMAT.
Display text strings raw, but preserve string syntax when replacing source
or when SOURCE-RESULT is non-nil."
  (unless (and (integerp lyric-result-width) (> lyric-result-width 0))
    (user-error "lyric-result-width must be a positive integer"))
  (let ((output (generate-new-buffer " *Lyric evaluation*"))
        (errors (get-buffer-create "*Lyric Errors*"))
        (command (format "%s --format %s --width %d%s"
                         lyric-vrsctl-command
                         (if editor-format "editor" "pretty")
                         lyric-result-width
                         (if (or replace source-result) "" " --raw"))))
    (unwind-protect
        (progn
          (with-current-buffer errors
            (let ((inhibit-read-only t)) (erase-buffer)))
          (message "Evaluating Lyric (C-g to cancel)…")
          (let ((status (lyric--run-region start end command output errors)))
            (unless (equal status 0)
              (display-buffer errors)
              (user-error "Lyric evaluation failed (status %s); see *Lyric Errors*" status))
            (let ((text (with-current-buffer output (buffer-string))))
              (if replace
                  ;; Only remove vrsctl's final record separator. Do not trim
                  ;; whitespace belonging to a raw/opaque result.
                  (let ((text (string-remove-suffix "\n" text)))
                    (atomic-change-group
                      (delete-region start end)
                      (goto-char start)
                      (let ((begin (point)))
                        (insert text)
                        (indent-region begin (point)))))
                (with-current-buffer (get-buffer-create "*Lyric Result*")
                  (let ((inhibit-read-only t))
                    (erase-buffer)
                    (insert text)
                    (lyric-mode)
                    (setq buffer-read-only t)
                    (goto-char (point-min)))
                  (display-buffer (current-buffer)))))))
      (kill-buffer output))))

(defun lyric-eval-buffer (editor-format)
  "Evaluate the current buffer with vrsctl.

With prefix argument EDITOR-FORMAT, request editor-formatted output."
  (interactive "P")
  (lyric--eval (point-min) (point-max) nil editor-format))

(defun lyric-eval-last-sexp (replace)
  "Evaluate the Lyric expression at its closing paren or preceding point.

With prefix argument REPLACE, replace the expression with its result."
  (interactive "P")
  (pcase-let ((`(,start . ,end) (lyric--last-sexp-bounds)))
    (lyric--eval start end replace)))

(defun lyric-eval-region (start end replace)
  "Evaluate Lyric source between START and END.

With prefix argument REPLACE, replace the region with its result."
  (interactive "r\nP")
  (lyric--eval start end replace))

(defun lyric-macroexpand-last-sexp (repeat-outer)
  "Display one expansion without executing the generated program.
The macro body runs and can perform effects.  With prefix REPEAT-OUTER,
expand the outermost call repeatedly.  This uses the
macro namespace of the vrsctl connection; for custom definitions, evaluate a
region containing both the definitions and an explicit macroexpand_1 call."
  (interactive "P")
  (let ((source (lyric--last-sexp-source))
        (lyric-vrsctl-command lyric-vrsctl-command)
        (lyric-result-width lyric-result-width))
    (with-temp-buffer
      (insert (format "(%s (quote %s))"
                      (if repeat-outer "macroexpand" "macroexpand_1") source))
      (lyric--eval (point-min) (point-max) nil nil t))))

(defvar-keymap lyric-mode-map
  :doc "Keymap for `lyric-mode'."
  "C-c C-c" #'lyric-eval-buffer
  "C-c C-e" #'lyric-eval-last-sexp
  "C-c C-m" #'lyric-macroexpand-last-sexp
  "C-c C-r" #'lyric-eval-region)

(define-derived-mode lyric-mode janet-mode "Lyric"
  "Major mode for editing Lyric programs."
  :syntax-table lyric-mode-syntax-table
  (setq-local indent-line-function #'lyric-indent-line)
  (setq-local lisp-indent-function #'lyric-indent-function)
  (setq-local indent-tabs-mode nil)
  (setq-local syntax-propertize-function #'lyric--syntax-propertize)
  (font-lock-add-keywords nil
                         '(("(\\(unquote-splicing\\|defmacro\\|for_syntax\\)\\_>" 1 font-lock-keyword-face)))
  (syntax-propertize (point-max)))

(add-to-list 'auto-mode-alist '("\\.ll\\'" . lyric-mode))

(provide 'lyric-mode)

;;; lyric-mode.el ends here
