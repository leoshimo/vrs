;;; vrs-choose.el --- Native VRS minibuffer choices -*- lexical-binding: t; -*-

(require 'vrs-mode)
(require 'cl-lib)

(defvar vrs--chooser-target nil
  "Buffer, modification tick, and bounds of the active native interaction.")

(defvar vrs--chooser-settings nil
  "Captured command and width, independent of buffer-local settings.")

(defun vrs--chooser-check ()
  "Reject a stale interaction before another request or source change."
  (pcase-let ((`(,buffer ,tick ,_start ,_end) vrs--chooser-target))
    (unless (buffer-live-p buffer)
      (user-error "The VRS source buffer was closed"))
    (with-current-buffer buffer
      (unless (= tick (buffer-chars-modified-tick))
        (user-error "The VRS source buffer changed; start the chooser again"))
      (barf-if-buffer-read-only))))

(defun vrs--chooser (bounds function)
  "Run FUNCTION against BOUNDS, retaining the source buffer's settings."
  (when vrs--chooser-target (user-error "A VRS chooser is already active"))
  (barf-if-buffer-read-only)
  (unless (and (integerp vrs-result-width) (> vrs-result-width 0))
    (user-error "vrs-result-width must be a positive integer"))
  (let ((vrs--chooser-target
         (list (current-buffer) (buffer-chars-modified-tick)
               (car bounds) (cdr bounds)))
        (vrs--chooser-settings (cons vrs-vrsctl-command vrs-result-width)))
    (funcall function)))

(defun vrs--chooser-request (source &optional raw)
  "Evaluate SOURCE in the shared session, returning text without its newline."
  (vrs--chooser-check)
  (let ((output (generate-new-buffer " *VRS choice*"))
        (errors (generate-new-buffer " *VRS choice errors*")))
    (unwind-protect
        (let ((status (with-temp-buffer
                        (insert source)
                        (vrs--run-region (point-min) (point-max)
                                         (car vrs--chooser-settings) output errors
                                         "compact" raw (cdr vrs--chooser-settings)))))
          (vrs--chooser-check)
          (unless (eq status 0)
            (user-error "VRS: %s" (with-current-buffer errors (string-trim (buffer-string)))))
          (with-current-buffer output (string-remove-suffix "\n" (buffer-string))))
      (kill-buffer output)
      (kill-buffer errors))))

(defun vrs--chooser-data (source)
  "Read a packet of lists and strings, never a Lyric value, from SOURCE."
  (let* ((text (vrs--chooser-request source))
         (read-circle nil)
         (parsed (read-from-string text)))
    (cl-labels ((packet-p (value)
                  (or (stringp value)
                      (and (proper-list-p value) (cl-every #'packet-p value)))))
      (unless (and (string-empty-p (string-trim (substring text (cdr parsed))))
                   (packet-p (car parsed)))
        (user-error "Invalid VRS chooser packet")))
    (car parsed)))

(defun vrs--lyric-string (text)
  "Quote TEXT with Lyric's supported escapes, without an Emacs Lisp printer."
  (concat "\"" (replace-regexp-in-string
                 "[\\\"\n\r\t]"
                 (lambda (match)
                   (cdr (assoc match '(("\\" . "\\\\") ("\"" . "\\\"")
                                       ("\n" . "\\n") ("\r" . "\\r") ("\t" . "\\t")))))
                 text t t)
          "\""))

(defun vrs--choose-row (prompt rows label detail category)
  "Choose an actual row from ROWS; optionally annotate it with DETAIL."
  (unless rows (user-error "No %s available" category))
  (let* ((index 0)
         (candidates
          (mapcar (lambda (row)
                    (cons (format "%s  [%d]"
                                  (replace-regexp-in-string "[\n\r\t]" " " (funcall label row))
                                  (cl-incf index))
                          row))
                  rows))
         (table
          (lambda (string predicate action)
            (if (eq action 'metadata)
                `(metadata
                  (category . ,category)
                  (annotation-function
                   . ,(lambda (candidate)
                        (if detail
                            (concat "  " (replace-regexp-in-string
                                          "[\n\r\t]" " "
                                          (funcall detail (cdr (assoc candidate candidates)))))
                          ""))))
              (complete-with-action action candidates string predicate))))
         (selection (completing-read prompt table nil t)))
    (vrs--chooser-check)
    (or (cdr (assoc selection candidates)) (user-error "No VRS choice selected"))))

(defun vrs--chooser-replace (source)
  "Commit SOURCE atomically after the whole interaction succeeds."
  (vrs--chooser-check)
  (pcase-let ((`(,buffer ,_tick ,start ,end) vrs--chooser-target))
    (with-current-buffer buffer
      (atomic-change-group
        (delete-region start end)
        (goto-char start)
        (insert source)
        (indent-region start (point))))))

(defun vrs--choice-bounds ()
  "Use the active region or the expression at its closing paren/before point."
  (if (use-region-p) (cons (region-beginning) (region-end))
    (vrs--last-sexp-bounds)))

(defun vrs--choice-source (bounds)
  "Wrap exact source at BOUNDS as one expression, including a multi-form region."
  (concat "(begin\n" (buffer-substring-no-properties (car bounds) (cdr bounds)) "\n)"))

(defun vrs--choose-value (fields)
  "Evaluate a value list or, with FIELDS, a record; retain the chosen value."
  (let* ((bounds (vrs--choice-bounds))
         (source (vrs--choice-source bounds)))
    (vrs--chooser
     bounds
     (lambda ()
       (let* ((rows (vrs--chooser-data
                     (format "(vrs/editor_choices %s %s)" source (if fields "true" "false"))))
              (row (vrs--choose-row (if fields "Field: " "Value: ") rows
                                    #'car (when fields #'cadr) 'vrs-value)))
         (vrs--chooser-replace (cadr row)))))))

;;;###autoload
(defun vrs-choose-value ()
  "Evaluate the region or expression, choose a list element, and replace source.
Use native completion.  Lists and literal symbols are retained with a quote."
  (interactive)
  (vrs--choose-value nil))

;;;###autoload
(defun vrs-choose-field ()
  "Evaluate the region or expression and replace it with a chosen record field.
Accept keyword/value records and tagged entities; use native completion."
  (interactive)
  (vrs--choose-value t))

(defun vrs--choose-function (rows prompt)
  "Choose a function from ROWS, displaying docs, service, and signature."
  (vrs--choose-row prompt rows
                   (lambda (row) (format "%s · %s · %s" (nth 3 row) (nth 2 row) (nth 1 row)))
                   nil 'vrs-function))

(defun vrs--read-argument (arg)
  "Choose a typed ARG or read source without evaluating that source."
  (pcase-let* ((`(,name ,type) arg)
               (rows (unless (string-empty-p type)
                       (vrs--chooser-data (format "(vrs/editor_argument %s)" type)))))
    (if rows
        (cadr (vrs--choose-row (format "%s (%s): " name type) rows #'car nil 'vrs-value))
      (let ((source (read-string (format "%s%s (Lyric expression): " name
                                        (if (string-empty-p type) "" (concat " · " type))))))
        (vrs--chooser-check)
        ;; read/pretty construct source; entering (some_action) never runs it.
        (vrs--chooser-request (format "(pretty (read %s) %d)"
                                      (vrs--lyric-string source) (cdr vrs--chooser-settings)) t)))))

(defun vrs--filled-call (row arguments)
  "Fill ROW's arguments after the supplied literal ARGUMENTS."
  (dolist (arg (nthcdr (length arguments) (nth 4 row)))
    (setq arguments (append arguments (list (vrs--read-argument arg)))))
  (concat "(" (car row)
          (if arguments (concat " " (string-join arguments " ")) "") ")"))

;;;###autoload
(defun vrs-browse-functions (&optional fill)
  "Insert a bound service call using native completion; do not execute it.
By default insert argument names as placeholders.  With prefix FILL, choose
arguments using entity completion providers, or enter unevaluated Lyric source."
  (interactive "P")
  (vrs--chooser
   (cons (point) (point))
   (lambda ()
     (let ((row (vrs--choose-function (vrs--chooser-data "(vrs/editor_functions)") "Function: ")))
       (vrs--chooser-replace (if fill (vrs--filled-call row nil) (nth 3 row)))))))

(defun vrs--act-on-value (execute)
  "Construct a call on the evaluated entity; run it only when EXECUTE is non-nil."
  (let* ((bounds (vrs--choice-bounds))
         (source (vrs--choice-source bounds)))
    (vrs--chooser
     bounds
     (lambda ()
       (let* ((entity (vrs--chooser-data (format "(vrs/editor_value %s)" source)))
              (literal (cadr entity))
              (row (vrs--choose-function
                    (vrs--chooser-data (format "(vrs/editor_actions %s)" literal))
                    (if execute "Execute action: " "Construct action: ")))
              (call (vrs--filled-call row (list literal))))
         (if execute
             (let ((result (vrs--chooser-request (format "(vrs/execute_command '%s)" call))))
               (with-current-buffer (get-buffer-create "*VRS Result*")
                 (let ((inhibit-read-only t))
                   (erase-buffer) (insert result "\n") (vrs-mode)
                   (setq buffer-read-only t) (goto-char (point-min)))
                 (display-buffer (current-buffer))))
           (vrs--chooser-replace call)))))))

;;;###autoload
(defun vrs-act-on-value ()
  "Choose an action for the region's or expression's entity and replace source.
Fill remaining arguments using native completion.  Do not execute the call."
  (interactive)
  (vrs--act-on-value nil))

;;;###autoload
(defun vrs-execute-action ()
  "Choose and execute an action on the region's or expression's entity.
Fill remaining arguments in the minibuffer.  Publish the concrete call to :cmd
for macro recording.  Leave source intact and display the result."
  (interactive)
  (vrs--act-on-value t))

(provide 'vrs-choose)
;;; vrs-choose.el ends here
