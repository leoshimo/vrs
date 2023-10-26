#!/usr/bin/env sh

for i in {1..100}; do
    (vrsctl -c "(loop (begin (send (self) :hi) (recv)))" >/dev/null 2>/dev/null) &
done
