#!/usr/bin/env osascript
# stickies_get - Get list of Stickies
#

tell application "System Events"
	set output to "("
	tell process "Stickies"
		set theWindows to every window
		repeat with w in every window
			set theName to title of w
			set output to output & "(:title " & "\"" & theName & "\"" & ")" & linefeed
		end repeat
	end tell
	set output to output & ")"
	return output
end tell
