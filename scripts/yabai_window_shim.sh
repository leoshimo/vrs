#!/usr/bin/env bash
# yabai_window_shim.sh - Shim for Yabai WM
#

# TODO: Investigate bugs in `read` that breaks escaped quotes in quotes occassionally
echo -n "("
yabai -m query --windows | \
    jq '[.[] | select(."is-visible" == true)]' | \
    jq -r '.[] | "(:id \(.id | @json) :app \(.app | @json) :title \(.title | @json))"' \
    | sed -E 's/\\"//g'
echo -n ")"
