#!/usr/bin/env osascript
# ax_raise.sh - Raise a window in given app with title
#
    
on run argv
    if (count of argv) < 2 then
        log "Usage: ax_raise APP WIN_NAME"
        return
    end if
    
    set appName to item 1 of argv
    set windowTitle to item 2 of argv

	tell application "System Events"
		tell process appName
			set theWindow to first window where name is windowTitle
            set frontmost to true
			perform action "AXRaise" of theWindow
		end tell
	end tell
end run
