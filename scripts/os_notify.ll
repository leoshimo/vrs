#!/usr/bin/env vrsctl
# os_notify.ll - OS Specific Notifications

(defn macOS? ()
  "Determine if current device is macOS"
  (eq? (get (exec "uname" "-s") 1) "Darwin"))

(defn macos_ui_notify (title message)
  "Show Notification UI for macOS"
  (exec "osascript" "-e"
        (format "display notification \"{}\" with title \"{}\"" message title)))

(defn linux_ui_notify (title message)
  "Show Notification UI for linux"
  (exec "notify-send" title message "--icon=dialog-information"))

(defn notify (title subtitle)
  "(notify TITLE SUBTITLE) - Show OS Desktop UI for notification"
  (if (macOS?)
    (macos_ui_notify title subtitle)
    (linux_ui_notify title subtitle)))

# Depends on shortcuts
(defn toggle_do_not_disturb ()
  (exec "shortcuts" "run" "do-not-disturb-toggle"))

(spawn_srv :os_notify :interface '(notify toggle_do_not_disturb))
