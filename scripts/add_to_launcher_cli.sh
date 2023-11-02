#!/usr/bin/env bash
# add_to_launcher.sh - CLI shell to add to launcher
#

set -eu

echo -n "Title: "
read
TITLE=$REPLY

echo -n "Cmd S-Exp: "
read
CMD=$REPLY

vrsctl -c "(call (find-srv :launcher) '(:add_item \"$TITLE\" $CMD))"
