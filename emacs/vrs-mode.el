;;; vrs-mode.el --- Major mode and evaluation helpers for Lyric -*- lexical-binding: t; -*-

(require 'lisp-mode)
(require 'subr-x)
(require 'json)

(defgroup vrs nil
  "Editing Lyric programs."
  :group 'languages)

(defcustom vrs-vrsctl-command "vrsctl"
  "Base vrsctl command used by Lyric evaluation commands.
Buffers using the same command share a persistent runtime session."
  :type 'string
  :group 'vrs)

(defcustom vrs-result-width 90
  "Target column width for evaluated values."
  :type '(integer :tag "Columns")
  :group 'vrs)

(defun vrs--prefixes-before (position)
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

(defun vrs--prefix-context (context prefix)
  "Apply PREFIX to CONTEXT, a (LITERAL DEPTH DATA) list."
  (pcase-let ((`(,literal ,depth ,data) context))
    (cond
     (literal context)
     ((and (equal prefix "'") (= depth 0)) (list t 0 t))
     ((equal prefix "`") (list nil (1+ depth) t))
     ((and (member prefix '("," ",@")) (> depth 0))
      (list nil (1- depth) (> depth 1)))
     (t (list literal depth data)))))

(defun vrs--list-context (open)
  "Return the quotation and data context of the list at OPEN."
  (save-excursion
    (goto-char open)
    (let* ((parent (nth 1 (syntax-ppss)))
           (context (if parent (vrs--list-context parent) (list nil 0 nil))))
      ;; Long reader forms have the same indentation boundaries as prefixes.
      (when parent
        (goto-char (1+ parent))
        (forward-comment (point-max))
        (when (looking-at "\\(quote\\|quasiquote\\|unquote-splicing\\|unquote\\)\\_>")
          (setq context
                (vrs--prefix-context
                 context (cdr (assoc (match-string-no-properties 1)
                                     '(("quote" . "'") ("quasiquote" . "`")
                                       ("unquote" . ",") ("unquote-splicing" . ",@"))))))))
      (dolist (prefix (cdr (vrs--prefixes-before open)))
        (setq context (vrs--prefix-context context prefix)))
      (goto-char (1+ open))
      (forward-comment (point-max))
      (setf (nth 2 context)
            (or (nth 0 context) (> (nth 1 context) 0) (nth 2 context)
                (memq (char-after) '(?: ?\( ?\" ?\' ?\)))
                (looking-at "\\(?:-?[0-9]+\\|nil\\|true\\|false\\)\\_>")))
      context)))

(defun vrs--data-list-p (open)
  "Whether the list at OPEN contains data rather than a function call."
  (nth 2 (vrs--list-context open)))

(defun vrs--indent-column ()
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
       ((vrs--data-list-p open)
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

(defun vrs-indent-line ()
  "Indent a Lyric line, preserving raw string contents."
  (interactive)
  (let ((column (vrs--indent-column))
        (offset (- (point-max) (point))))
    (when column
      (indent-line-to column)
      (when (> (- (point-max) offset) (point))
        (goto-char (- (point-max) offset))))))

(defun vrs-indent-function (indent-point _state)
  "Use Lyric indentation at INDENT-POINT for `indent-sexp' too."
  (save-excursion
    (goto-char indent-point)
    (vrs--indent-column)))

(defvar vrs-mode-syntax-table
  (let ((table (make-syntax-table)))
    (modify-syntax-entry ?\( "()" table)
    (modify-syntax-entry ?\) ")(" table)
    (modify-syntax-entry ?\" "\"" table)
    (modify-syntax-entry ?\\ "\\" table)
    (modify-syntax-entry ?# "<" table)
    (modify-syntax-entry ?\n ">" table)
    (dolist (char (string-to-list "!$%&*+-./:;<=>?@[]^_{|}~"))
      (modify-syntax-entry char "_" table))
    (modify-syntax-entry ?` "'" table)
    (modify-syntax-entry ?, "'" table)
    (modify-syntax-entry ?\' "'" table)
    table)
  "Syntax table used in `vrs-mode'.")

(defconst vrs-font-lock-keywords
  `((,(concat "(" (regexp-opt '("begin" "cond" "def" "defmacro" "eval"
                                "fn" "for_syntax" "if" "lambda" "let" "loop"
                                "match" "quasiquote" "quote" "set" "try"
                                "unquote" "unquote-splicing" "yield") t)
              "\\_>")
     1 font-lock-keyword-face)
    ("(\\(\\(?:\\sw\\|\\s_\\)+!\\)\\_>" 1 font-lock-keyword-face)
    ("(\\(?:defn!\\|defmacro\\)\\_>[ \t]+\\(\\(?:\\sw\\|\\s_\\)+\\)"
     1 font-lock-function-name-face)
    ("(def\\_>[ \t]+\\(\\(?:\\sw\\|\\s_\\)+\\)"
     1 font-lock-variable-name-face)
    ("\\_<:\\(?:\\sw\\|\\s_\\)*\\_>" . font-lock-constant-face)
    (,(regexp-opt '("nil" "true" "false") 'symbols) . font-lock-constant-face)
    ("\\_<-?[0-9]+\\_>" . font-lock-constant-face))
  "Highlight Lyric forms, definitions, and literal values.")

(defun vrs--syntax-propertize (_start _end)
  "Mark reader prefixes, strings, and symbol contents in the current buffer."
  (save-excursion
    (with-silent-modifications
      (remove-text-properties (point-min) (point-max)
                              '(syntax-table nil syntax-multiline nil))
      (syntax-ppss-flush-cache (point-min))
      (goto-char (point-min))
      (while (< (point) (point-max))
        (skip-chars-forward " \t\r\n\f\v")
        (let ((start (point)))
          (cond
           ((eobp))
           ((eq (char-after) ?#) (forward-line 1))
           ((looking-at ",@")
            (put-text-property (1+ start) (+ start 2)
                               'syntax-table (string-to-syntax "'"))
            (forward-char 2))
           ((memq (char-after) '(?\( ?\) ?\' ?` ?,)) (forward-char))
           ((looking-at "\"\"\"")
            (forward-char 3)
            ;; The first three opening quotes and last three closing quotes
            ;; delimit a raw block; backslashes within it have no escape role.
            (let ((closed (re-search-forward "\"\\{3,\\}" nil t)))
              (unless closed (goto-char (point-max)))
              (put-text-property start (point) 'syntax-table (string-to-syntax "."))
              (put-text-property start (1+ start) 'syntax-table (string-to-syntax "|"))
              (when closed
                (put-text-property (1- (point)) (point)
                                   'syntax-table (string-to-syntax "|")))
              (put-text-property start (point) 'syntax-multiline t)))
           ((eq (char-after) ?\")
            (forward-char)
            (let (closed)
              (while (and (not closed) (not (eobp)))
                (skip-chars-forward "^\"\\\\")
                (cond
                 ((eq (char-after) ?\\) (forward-char (min 2 (- (point-max) (point)))))
                 ((eq (char-after) ?\") (forward-char) (setq closed t))))))
           (t
            (skip-chars-forward "^ \t\r\n\f\v()'`,")
            ;; In symbols, # and quotes are ordinary characters.  The reader
            ;; recognizes comments and strings only at the start of a token.
            (let ((end (point)))
              (save-excursion
                (goto-char start)
                (while (re-search-forward "[#\"\\]" end t)
                  (put-text-property (1- (point)) (point)
                                     'syntax-table (string-to-syntax "_"))))))))))))

(defun vrs--last-sexp-bounds ()
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
      (goto-char (car (vrs--prefixes-before (point))))
      (cons (point) end))))

(defvar vrs--sessions (make-hash-table :test #'equal)
  "Persistent vrsctl processes, keyed by their base command.")

(defun vrs--close-session (command)
  "Close COMMAND's connection and discard its session state."
  (when-let* ((process (gethash command vrs--sessions)))
    (remhash command vrs--sessions)
    (dolist (child (list process (process-get process 'stderr)))
      (when (and child (process-live-p child)) (delete-process child)))
    (when-let* ((buffer (process-get process 'errors)))
      (when (buffer-live-p buffer) (kill-buffer buffer)))))

(defun vrs--session-filter (process chunk)
  "Collect complete JSON replies from PROCESS, including split CHUNKs."
  (let ((text (concat (process-get process 'partial) chunk)))
    (while (string-match "\n" text)
      (let ((line (substring text 0 (match-beginning 0))))
        (setq text (substring text (match-end 0)))
        (condition-case err
            (process-put process 'response
                         (json-parse-string line :object-type 'plist
                                            :false-object :false :null-object nil))
          (error
           (process-put process 'response
                        (list :ok :false :error
                              (format "Invalid vrsctl session reply: %s"
                                      (error-message-string err))))
           (delete-process process)))))
    (process-put process 'partial text)))

(defun vrs--session (command)
  "Return COMMAND's live session, starting one when needed."
  (let ((process (gethash command vrs--sessions)))
    (unless (and process (process-live-p process))
      (vrs--close-session command)
      (let* ((errors (generate-new-buffer " *VRS session errors*"))
             (stderr (make-pipe-process :name "VRS errors" :buffer errors
                                        :noquery t :sentinel #'ignore)))
        (condition-case err
            (setq process
                  (make-process :name "VRS session"
                                :command (list shell-file-name shell-command-switch
                                               (concat "exec " command " --session"))
                                :connection-type 'pipe :coding 'utf-8-unix
                                :filter #'vrs--session-filter :stderr stderr
                                :noquery t :sentinel #'ignore))
          (error
           (delete-process stderr)
           (kill-buffer errors)
           (signal (car err) (cdr err))))
        (process-put process 'errors errors)
        (process-put process 'stderr stderr)
        (process-put process 'partial "")
        (puthash command process vrs--sessions)))
    process))

(defun vrs--run-region (start end command output errors &optional format raw width)
  "Evaluate START to END in COMMAND's session, collecting OUTPUT and ERRORS.
FORMAT, RAW, and WIDTH apply to this request.  C-g closes the connection;
ordinary evaluation errors leave the session available."
  (let ((process (vrs--session command))
        (inhibit-quit nil)
        completed)
    (when (process-get process 'busy)
      (user-error "This VRS session is busy; finish or cancel its current evaluation"))
    (process-put process 'busy t)
    (process-put process 'response nil)
    (unwind-protect
        (progn
          (with-current-buffer (process-get process 'errors) (erase-buffer))
          (process-send-string
           process
           (concat (json-serialize
                    (list :source (buffer-substring-no-properties start end)
                          :format (or format "pretty") :width (or width 90)
                          :raw (if raw t :false))
                    :false-object :false)
                   "\n"))
          (while (and (process-live-p process)
                      (not (process-get process 'response)))
            (accept-process-output process 0.05))
          (while (accept-process-output (process-get process 'stderr) 0.01))
          (let* ((reply (process-get process 'response))
                 (ok (and (eq (plist-get reply :ok) t)
                          (stringp (plist-get reply :output)))))
            (if ok
                (with-current-buffer output (insert (plist-get reply :output)))
              (let ((diagnostic (with-current-buffer (process-get process 'errors)
                                  (buffer-string))))
                (with-current-buffer errors
                  (insert (or (plist-get reply :error)
                              (unless (string-empty-p diagnostic) diagnostic)
                              "VRS connection closed; the next evaluation starts a fresh session.")
                          "\n"))))
            (setq completed t)
            (if ok 0 1)))
      (process-put process 'busy nil)
      (unless (and completed (process-live-p process))
        (vrs--close-session command)))))

(defun vrs-reset-session ()
  "Start a fresh vrsctl connection, clearing this session's definitions.
All buffers using the same `vrs-vrsctl-command' share this reset."
  (interactive)
  (let ((command vrs-vrsctl-command)
        (output (generate-new-buffer " *VRS reset*"))
        (errors (generate-new-buffer " *VRS reset errors*")))
    (vrs--close-session command)
    (unwind-protect
        (with-temp-buffer
          (insert "nil")
          (unless (zerop (vrs--run-region (point-min) (point-max) command output errors))
            (user-error "%s" (with-current-buffer errors (buffer-string))))
          (message "Started a fresh VRS session"))
      (kill-buffer output)
      (kill-buffer errors))))

(defun vrs--last-sexp-source ()
  "Return the exact Lyric expression preceding point."
  (pcase-let ((`(,start . ,end) (vrs--last-sexp-bounds)))
    (buffer-substring-no-properties start end)))

(defun vrs--eval (start end replace &optional editor-format source-result)
  "Evaluate START to END; optionally REPLACE or request EDITOR-FORMAT.
Display text strings raw, but preserve string syntax when replacing source
or when SOURCE-RESULT is non-nil."
  (unless (and (integerp vrs-result-width) (> vrs-result-width 0))
    (user-error "vrs-result-width must be a positive integer"))
  (let ((output (generate-new-buffer " *VRS evaluation*"))
        (errors (get-buffer-create "*VRS Errors*"))
        (command vrs-vrsctl-command))
    (unwind-protect
        (progn
          (with-current-buffer errors
            (let ((inhibit-read-only t)) (erase-buffer)))
          (message "Evaluating VRS (C-g to cancel and reset session)…")
          (let ((status (vrs--run-region start end command output errors
                                        (if editor-format "editor" "pretty")
                                        (not (or replace source-result))
                                        vrs-result-width)))
            (unless (equal status 0)
              (display-buffer errors)
              (user-error "VRS evaluation failed (status %s); see *VRS Errors*" status))
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
                (with-current-buffer (get-buffer-create "*VRS Result*")
                  (let ((inhibit-read-only t))
                    (erase-buffer)
                    (insert text)
                    (vrs-mode)
                    (setq buffer-read-only t)
                    (goto-char (point-min)))
                  (display-buffer (current-buffer)))))))
      (kill-buffer output))))

(defun vrs-eval-buffer (editor-format)
  "Evaluate the current buffer with vrsctl.

With prefix argument EDITOR-FORMAT, request editor-formatted output."
  (interactive "P")
  (vrs--eval (point-min) (point-max) nil editor-format))

(defun vrs-eval-last-sexp (replace)
  "Evaluate the Lyric expression at its closing paren or preceding point.

With prefix argument REPLACE, replace the expression with its result."
  (interactive "P")
  (pcase-let ((`(,start . ,end) (vrs--last-sexp-bounds)))
    (vrs--eval start end replace)))

(defun vrs-eval-region (start end replace)
  "Evaluate Lyric source between START and END.

With prefix argument REPLACE, replace the region with its result."
  (interactive "r\nP")
  (vrs--eval start end replace))

(defun vrs-browse-functions ()
  "Open vrsjmp to choose a service call and insert it at point.
Press Enter in vrsjmp for argument placeholders, or use its Fill arguments
action to choose values.  The selected call is not executed.  Cancelling
with C-g or leaving the picker keeps the buffer unchanged."
  (interactive)
  (barf-if-buffer-read-only)
  (let ((target (copy-marker (point) t))
        (command vrs-vrsctl-command)
        (width vrs-result-width))
    (unwind-protect
        (let ((text (with-temp-buffer
                      (vrs-mode)
                      (insert "(vrsjmp_browse_functions)")
                      (let ((vrs-vrsctl-command command)
                            (vrs-result-width width))
                        (vrs--eval (point-min) (point-max) t))
                      (buffer-string))))
          (unless (marker-buffer target)
            (user-error "The insertion buffer was closed"))
          (with-current-buffer (marker-buffer target)
            (goto-char target)
            (atomic-change-group
              (let ((start (point)))
                (insert text)
                (indent-region start (point))))))
      (set-marker target nil))))

(defun vrs-macroexpand-last-sexp (repeat-outer)
  "Display one expansion without executing the generated program.
The macro body runs and can perform effects.  With prefix REPEAT-OUTER,
expand the outermost call repeatedly.  Earlier evaluated definitions in
the shared VRS session are available during expansion."
  (interactive "P")
  (let ((source (vrs--last-sexp-source))
        (command vrs-vrsctl-command)
        (width vrs-result-width))
    (with-temp-buffer
      (insert (format "(%s (quote %s))"
                      (if repeat-outer "macroexpand" "macroexpand_1") source))
      (let ((vrs-vrsctl-command command)
            (vrs-result-width width))
        (vrs--eval (point-min) (point-max) nil nil t)))))

(defvar vrs-mode-map
  (let ((map (make-sparse-keymap)))
    (define-key map (kbd "C-M-q") #'indent-sexp)
    (define-key map (kbd "C-c C-c") #'vrs-eval-buffer)
    (define-key map (kbd "C-c C-e") #'vrs-eval-last-sexp)
    (define-key map (kbd "C-c C-m") #'vrs-macroexpand-last-sexp)
    (define-key map (kbd "C-c C-r") #'vrs-eval-region)
    map)
  "Keymap for `vrs-mode'.")

(define-derived-mode vrs-mode prog-mode "VRS"
  "Major mode for editing Lyric programs and evaluating them with vrsctl."
  :syntax-table vrs-mode-syntax-table
  (setq-local font-lock-defaults '(vrs-font-lock-keywords))
  (setq-local comment-start "# ")
  (setq-local comment-end "")
  (setq-local comment-start-skip "#+[ \t]*")
  (setq-local parse-sexp-ignore-comments t)
  (setq-local indent-line-function #'vrs-indent-line)
  (setq-local lisp-indent-function #'vrs-indent-function)
  (setq-local indent-tabs-mode nil)
  (setq-local syntax-propertize-function #'vrs--syntax-propertize)
  (syntax-propertize (point-max)))

(add-to-list 'auto-mode-alist '("\\.ll\\'" . vrs-mode))

(provide 'vrs-mode)

;;; vrs-mode.el ends here
