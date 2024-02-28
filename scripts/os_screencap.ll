#!/usr/bin/env vrsctl
# os_screencap.ll - Screen Capture Service
#

# AppleScript Ref: https://apple.stackexchange.com/questions/374076/how-to-screen-record-using-applescript-on-catalina

(defn start_screencap ()
  "(start_recording) - Start a Screen Recording if recording is not active"
  (exec "osascript" "-e" "tell application \"QuickTime Player\" to new screen recording"))

(spawn_srv :os_screencap :interface '(start_screencap))
