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

(defcustom lyric-result-width 80
  "Target column width for evaluated values."
  :type '(integer :tag "Columns")
  :group 'lyric)

(defun lyric--data-list-p (open)
  "Whether the list at OPEN contains data rather than a function call."
  (save-excursion
    (goto-char open)
    (or (eq (char-before) ?\')
        ;; Quoted data remains data at any nesting depth.
        (let ((parent (nth 1 (syntax-ppss))))
          (and parent (lyric--data-list-p parent)))
        (progn
          (forward-char)
          (skip-chars-forward " \t\n")
          (or (memq (char-after) '(?: ?\( ?\" ?\' ?\)))
              (looking-at "\\(?:-?[0-9]+\\|nil\\|true\\|false\\)\\_>"))))))

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
                    (member head '("begin" "defn" "lambda" "loop")))
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
    ;; Janet uses backticks as string delimiters; Lyric does not.
    (modify-syntax-entry ?` "." table)
    ;; Include Lyric's quote prefix in backward-sexp/evaluation bounds.
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
              (syntax-ppss-flush-cache run-start))))))))

(defun lyric--last-sexp-bounds ()
  "Return the bounds of the Lyric expression preceding point.

Unlike `pp-last-sexp', this preserves the exact source text, including raw
block strings, rather than reading and printing it as Emacs Lisp."
  (save-excursion
    (skip-chars-backward " \t\r\n")
    (let ((end (point)))
      (backward-sexp)
      (cons (point) end))))

(defun lyric--last-sexp-source ()
  "Return the exact Lyric expression preceding point."
  (pcase-let ((`(,start . ,end) (lyric--last-sexp-bounds)))
    (buffer-substring-no-properties start end)))

(defun lyric--eval (start end replace &optional editor-format)
  "Evaluate START to END; optionally REPLACE or request EDITOR-FORMAT.
Display text strings raw, but preserve string syntax when replacing source."
  (unless (and (integerp lyric-result-width) (> lyric-result-width 0))
    (user-error "lyric-result-width must be a positive integer"))
  (let ((output (generate-new-buffer " *Lyric evaluation*"))
        (errors (get-buffer-create "*Lyric Errors*"))
        (command (format "%s --format %s --width %d%s"
                         lyric-vrsctl-command
                         (if editor-format "editor" "pretty")
                         lyric-result-width
                         (if replace "" " --raw"))))
    (unwind-protect
        (let ((status (shell-command-on-region
                       start end command output nil errors)))
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
                (display-buffer (current-buffer))))))
      (kill-buffer output))))

(defun lyric-eval-buffer (editor-format)
  "Evaluate the current buffer with vrsctl.

With prefix argument EDITOR-FORMAT, request editor-formatted output."
  (interactive "P")
  (lyric--eval (point-min) (point-max) nil editor-format))

(defun lyric-eval-last-sexp (replace)
  "Evaluate the Lyric expression preceding point.

With prefix argument REPLACE, replace the expression with its result."
  (interactive "P")
  (pcase-let ((`(,start . ,end) (lyric--last-sexp-bounds)))
    (lyric--eval start end replace)))

(defun lyric-eval-region (start end replace)
  "Evaluate Lyric source between START and END.

With prefix argument REPLACE, replace the region with its result."
  (interactive "r\nP")
  (lyric--eval start end replace))

(defvar-keymap lyric-mode-map
  :doc "Keymap for `lyric-mode'."
  "C-c C-c" #'lyric-eval-buffer
  "C-c C-e" #'lyric-eval-last-sexp
  "C-c C-r" #'lyric-eval-region)

(define-derived-mode lyric-mode janet-mode "Lyric"
  "Major mode for editing Lyric programs."
  :syntax-table lyric-mode-syntax-table
  (setq-local indent-line-function #'lyric-indent-line)
  (setq-local lisp-indent-function #'lyric-indent-function)
  (setq-local indent-tabs-mode nil)
  (setq-local syntax-propertize-function #'lyric--syntax-propertize)
  (syntax-propertize (point-max)))

(add-to-list 'auto-mode-alist '("\\.ll\\'" . lyric-mode))

(provide 'lyric-mode)

;;; lyric-mode.el ends here
