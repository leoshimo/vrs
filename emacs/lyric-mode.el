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

(defvar lyric-mode-syntax-table
  (let ((table (copy-syntax-table janet-mode-syntax-table)))
    ;; Janet uses backticks as string delimiters; Lyric does not.
    (modify-syntax-entry ?` "." table)
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

(defun lyric-eval-buffer (editor-format)
  "Evaluate the current buffer with vrsctl.

With prefix argument EDITOR-FORMAT, request editor-formatted output."
  (interactive "P")
  (shell-command-on-region
   (point-min) (point-max)
   (if editor-format
       (concat lyric-vrsctl-command " --format editor")
     lyric-vrsctl-command)))

(defun lyric-eval-last-sexp (replace)
  "Evaluate the Lyric expression preceding point.

With prefix argument REPLACE, replace the expression with its result."
  (interactive "P")
  (let* ((bounds (lyric--last-sexp-bounds))
         (source (buffer-substring-no-properties (car bounds) (cdr bounds)))
         (argument (shell-quote-argument source))
         (command (concat lyric-vrsctl-command " --command " argument)))
    (if replace
        (let ((result (string-trim (shell-command-to-string command))))
          (delete-region (car bounds) (cdr bounds))
          (goto-char (car bounds))
          (insert result))
      (shell-command command))))

(defun lyric-eval-region (start end replace)
  "Evaluate Lyric source between START and END.

With prefix argument REPLACE, replace the region with its result."
  (interactive "r\nP")
  (shell-command-on-region start end lyric-vrsctl-command nil replace))

(defvar-keymap lyric-mode-map
  :doc "Keymap for `lyric-mode'."
  "C-c C-c" #'lyric-eval-buffer
  "C-c C-e" #'lyric-eval-last-sexp
  "C-c C-r" #'lyric-eval-region)

(define-derived-mode lyric-mode janet-mode "Lyric"
  "Major mode for editing Lyric programs."
  :syntax-table lyric-mode-syntax-table
  (setq-local syntax-propertize-function #'lyric--syntax-propertize)
  (syntax-propertize (point-max)))

(add-to-list 'auto-mode-alist '("\\.ll\\'" . lyric-mode))

(provide 'lyric-mode)

;;; lyric-mode.el ends here
