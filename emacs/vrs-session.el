;;; vrs-session.el --- Local and remote editor sessions -*- lexical-binding: t; -*-

(require 'cl-lib)
(require 'json)
(require 'subr-x)

(defvar vrs-vrsctl-command)

(cl-defstruct (vrs--editor-session (:constructor vrs--make-editor-session))
	      base node name started)

(defvar vrs--sessions (make-hash-table :test #'eq)
  "Live vrsctl processes, keyed by editor session objects.")

(defvar vrs--session-list nil "Editor-owned session identities.")
(defvar-local vrs--selected-session nil "Session selected for this buffer.")
(defvar vrs--last-session nil "Session last used in a VRS buffer.")
(defvar vrs--local-nodes (make-hash-table :test #'equal))
(defvar vrs--identifying-node nil)

(defun vrs--get-session (base node &optional name)
  "Get or create the editor session named NAME on NODE, using BASE CLI."
  (or (cl-find-if (lambda (session)
                    (and (equal base (vrs--editor-session-base session))
                         (equal node (vrs--editor-session-node session))
                         (equal name (vrs--editor-session-name session))))
                  vrs--session-list)
      (let ((session (vrs--make-editor-session :base base :node node :name name)))
        (push session vrs--session-list)
        session)))

(defun vrs--current-session (&optional base)
  "Return this buffer's session, inheriting the last session for BASE."
  (setq base (or base vrs-vrsctl-command))
  (unless (and vrs--selected-session
               (equal base (vrs--editor-session-base vrs--selected-session)))
    (setq vrs--selected-session
          (if (and vrs--last-session
                   (equal base (vrs--editor-session-base vrs--last-session)))
              vrs--last-session
            (vrs--get-session base nil))))
  vrs--selected-session)

(defun vrs--remember-session ()
  "Remember the active VRS buffer's session for newly opened buffers."
  (when (derived-mode-p 'vrs-mode)
    (setq vrs--last-session (vrs--current-session))))

(defun vrs--session-label (session)
  (concat (or (vrs--editor-session-node session)
              (gethash (vrs--editor-session-base session) vrs--local-nodes)
              "local")
          (when-let* ((name (vrs--editor-session-name session))) (concat "/" name))))

(defun vrs--session-command (session command)
  "Build the CLI invocation of COMMAND for SESSION."
  (concat "exec " (vrs--editor-session-base session) " " command
          (when-let* ((node (vrs--editor-session-node session)))
            (concat " --node " (shell-quote-argument node)))))

(defun vrs--session-sentinel (process _event)
  (when-let* ((session (process-get process 'session)))
    (unless (or (process-live-p process) (vrs--editor-session-node session))
      (remhash (vrs--editor-session-base session) vrs--local-nodes)))
  (force-mode-line-update t))

(defun vrs--mode-name ()
  "Compact session label with activity or disconnect indication."
  (let* ((session (vrs--current-session))
         (process (gethash session vrs--sessions)))
    (concat "VRS[" (vrs--session-label session)
            (cond ((and process (process-live-p process))
                   (if (process-get process 'busy) "…" ""))
                  ((vrs--editor-session-started session) "!")
                  (t ""))
            "]")))

(defun vrs--session-evaluate (session source)
  "Evaluate SOURCE in SESSION and return its compact printed result."
  (let ((output (generate-new-buffer " *VRS request*"))
        (errors (generate-new-buffer " *VRS request errors*")))
    (unwind-protect
        (with-temp-buffer
          (insert source)
          (unless (zerop (vrs--run-region (point-min) (point-max) session output errors "compact"))
            (user-error "%s" (with-current-buffer errors (string-trim (buffer-string)))))
          (with-current-buffer output (string-trim-right (buffer-string))))
      (kill-buffer output)
      (kill-buffer errors))))

(defun vrs--discover-nodes ()
  "Return (LOCAL . NODES) using the local runtime's connection snapshot."
  (let* ((session (vrs--get-session vrs-vrsctl-command nil))
         (text (vrs--session-evaluate session "(ls_nodes)"))
         ;; Only read a checked list of strings, never arbitrary runtime data.
         (read-circle nil)
         (packet (read-from-string text))
         (nodes (car packet)))
    (unless (and (= (cdr packet) (length text))
                 (proper-list-p nodes) (cl-every #'stringp nodes))
      (user-error "Invalid VRS node list"))
    (cons (gethash vrs-vrsctl-command vrs--local-nodes) nodes)))

(defun vrs--check-session-name (name)
  (unless (and (not (string-empty-p name))
               (not (string-match-p "[/[:cntrl:]]" name)))
    (user-error "Session names must be nonempty and contain no slash or control characters")))

(defun vrs--fresh-session (base node &optional name)
  "Create an independently connected session; allocate a numeric name if absent."
  (let ((index 1))
    (if (and name (not (string-empty-p name)))
        (vrs--check-session-name name)
      (setq name nil)
      (while (not name)
        (let ((candidate (number-to-string index)))
          (unless (cl-find-if (lambda (s) (and (equal base (vrs--editor-session-base s))
                                               (equal node (vrs--editor-session-node s))
                                               (equal candidate (vrs--editor-session-name s))))
                              vrs--session-list)
            (setq name candidate)))
        (cl-incf index))))
  (when (cl-find-if (lambda (s) (and (equal base (vrs--editor-session-base s))
                                     (equal node (vrs--editor-session-node s))
                                     (equal name (vrs--editor-session-name s))))
                    vrs--session-list)
    (user-error "A session named %s already exists on this node" name))
  (vrs--get-session base node name))

(defun vrs-select-session (create)
  "Select a session, or enter NODE/NAME to create one.
With prefix CREATE, choose a node and optionally name a fresh session."
  (interactive "P")
  (pcase-let* ((`(,local . ,nodes) (vrs--discover-nodes))
               (base vrs-vrsctl-command)
               (current (vrs--current-session))
               (sessions (cl-remove-if-not
                          (lambda (s) (equal base (vrs--editor-session-base s)))
                          vrs--session-list)))
    (let ((selected
           (if create
               (let* ((node (completing-read "VRS node: " nodes nil t nil nil local))
                      (name (read-string "Session name (optional): ")))
                 (vrs--fresh-session base (unless (equal node local) node) name))
             (let* ((labels (delete-dups (append nodes (mapcar #'vrs--session-label sessions))))
                    (annotation
                     (lambda (label)
                       (concat (if (equal label (vrs--session-label current)) " *" "")
                               (if (equal (car (split-string label "/")) local) " local" ""))))
                    (table
                     (lambda (string predicate action)
                       (if (eq action 'metadata)
                           `(metadata (category . vrs-session)
                                      (annotation-function . ,annotation))
                         (complete-with-action action labels string predicate))))
                    (label (completing-read "VRS session: " table nil nil nil nil (vrs--session-label current)))
                    (existing (cl-find label sessions :key #'vrs--session-label :test #'equal)))
               (or existing
                   (pcase-let* ((parts (split-string label "/"))
                                (node (car parts)) (name (cadr parts)))
                     (unless (and (<= (length parts) 2) (member node nodes))
                       (user-error "Choose a connected node, optionally followed by /NAME"))
                     (when name (vrs--check-session-name name))
                     (vrs--get-session base (unless (equal node local) node) name)))))))
      ;; Establish the target before committing the selection; never fall back locally.
      (vrs--session-evaluate selected "nil")
      (setq vrs--selected-session selected vrs--last-session selected)
      (force-mode-line-update t)
      (message "VRS session %s" (vrs--session-label selected)))))

(defun vrs-rename-session (name)
  "Rename this session without changing its connection or other buffers."
  (interactive (list (read-string "Session name: " (vrs--editor-session-name (vrs--current-session)))))
  (vrs--check-session-name name)
  (let ((session (vrs--current-session)))
    (when (cl-find-if (lambda (s) (and (not (eq s session))
                                       (equal (vrs--editor-session-base s) (vrs--editor-session-base session))
                                       (equal (vrs--editor-session-node s) (vrs--editor-session-node session))
                                       (equal name (vrs--editor-session-name s))))
                      vrs--session-list)
      (user-error "A session named %s already exists on this node" name))
    (setf (vrs--editor-session-name session) name)
    (force-mode-line-update t)))

(defun vrs-open-debugger ()
  "Open the web debugger for the selected session's node."
  (interactive)
  (let ((session (vrs--current-session)))
    (make-process :name "VRS debugger" :buffer (get-buffer-create "*VRS Debugger*")
                  :command (list shell-file-name shell-command-switch
                                 (vrs--session-command session "dbg --web"))
                  :noquery t)))

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
                        (list :type "error" :message
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
                                        :noquery t :sentinel #'vrs--session-sentinel)))
        (condition-case err
            (setq process
                  (make-process :name "VRS session"
                                :command (list shell-file-name shell-command-switch
                                               (vrs--session-command command "rpc"))
                                :connection-type 'pipe :coding 'utf-8-unix
                                :filter #'vrs--session-filter :stderr stderr
                                :noquery t :sentinel #'vrs--session-sentinel))
          (error
           (delete-process stderr)
           (kill-buffer errors)
           (signal (car err) (cdr err))))
        (process-put process 'errors errors)
        (process-put process 'stderr stderr)
        (process-put process 'partial "")
        (process-put process 'session command)
        (setf (vrs--editor-session-started command) t)
        (puthash command process vrs--sessions)
        (force-mode-line-update t)))
    process))

(defun vrs--run-region (start end command output errors &optional format width literal)
  "Evaluate START to END in COMMAND's session, collecting OUTPUT and ERRORS.
FORMAT, WIDTH, and LITERAL apply to this request.  LITERAL requests
source that retains the returned value.  C-g closes the connection;
ordinary evaluation errors leave the session available."
  (unless (or vrs--identifying-node
              (vrs--editor-session-node command)
              (gethash (vrs--editor-session-base command) vrs--local-nodes))
    (let ((vrs--identifying-node t))
      (let* ((text (vrs--session-evaluate command "(node_name)"))
             (node (json-parse-string text)))
        (unless (stringp node) (user-error "Invalid VRS node name"))
        (puthash (vrs--editor-session-base command) node vrs--local-nodes))))
  (let ((process (vrs--session command))
        (inhibit-quit nil)
        pending-input
        completed)
    (when (process-get process 'busy)
      (user-error "This VRS session is busy; finish or cancel its current evaluation"))
    (process-put process 'busy t)
    (force-mode-line-update t)
    (process-put process 'response nil)
    (unwind-protect
        (progn
          (with-current-buffer (process-get process 'errors) (erase-buffer))
          (process-send-string
           process
           (concat (json-serialize
                    (list :source (buffer-substring-no-properties start end)
                          :file (or buffer-file-name (format "<buffer:%s>" (buffer-name)))
                          :line (line-number-at-pos start t)
                          :column (save-excursion (goto-char start) (1+ (- (point) (line-beginning-position))))
                          :format (or format "pretty") :width (or width 90)
                          :literal (if literal t :false))
                    :false-object :false)
                   "\n"))
          (while (and (process-live-p process)
                      (not (process-get process 'response)))
            (accept-process-output process 0.05)
            ;; A daemon's terminal frames need keyboard input read explicitly;
            ;; waiting only on the subprocess can leave C-g unprocessed.
            (unless noninteractive
              (when-let* ((event (read-event nil nil 0.01)))
                (if (eq event ?\C-g)
                    (signal 'quit nil)
                  (push event pending-input)))))
          (while (accept-process-output (process-get process 'stderr) 0.01))
          (let* ((reply (process-get process 'response))
                 (ok (and (equal (plist-get reply :type) "result")
                          (stringp (plist-get reply :output)))))
            (if ok
                (with-current-buffer output (insert (plist-get reply :output)))
              (let ((diagnostic (with-current-buffer (process-get process 'errors)
                                  (buffer-string))))
                (with-current-buffer errors
                  (insert (or (plist-get reply :message)
                              (unless (string-empty-p diagnostic) diagnostic)
                              "VRS connection closed; the next evaluation starts a fresh session.")
                          "\n"))))
            (setq completed t)
            (if ok 0 1)))
      (process-put process 'busy nil)
      (force-mode-line-update t)
      (when completed
        (setq unread-command-events
              (nconc (nreverse pending-input) unread-command-events)))
      (unless (and completed (process-live-p process))
        (vrs--close-session command)))))

(defun vrs-reset-session ()
  "Clear the selected session's state for every buffer using it."
  (interactive)
  (let ((session (vrs--current-session)))
    (vrs--close-session session)
    (vrs--session-evaluate session "nil")
    (message "Reset VRS session %s" (vrs--session-label session))))

(provide 'vrs-session)
;;; vrs-session.el ends here
