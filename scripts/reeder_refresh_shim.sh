#!/usr/bin/env bash
# reeder_refresh_shim.sh
#

# TODO: Shell escaping makes this tricky to do from lyric itself atm.

printf "("
shortcuts run "get-unread-reeder" | jq -r '.[] | "(:title \"\(.title)\" :url \"\(.url)\")"'
printf ")"
