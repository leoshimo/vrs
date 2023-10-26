#!/usr/bin/env sh

for i in {1..100}; do
    (vrsctl -c "(loop (sleep 10) (+ 1 1))" >/dev/null 2>/dev/null) &
done
