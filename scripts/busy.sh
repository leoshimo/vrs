#!/usr/bin/env sh

NUM=100
echo "Spawning $NUM"

for i in {1..$NUM}; do
    vrsctl -c "(loop (begin (send (self) :hi) (recv)))" >/dev/null 2>/dev/null &
done
