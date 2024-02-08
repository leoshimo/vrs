#!/usr/bin/env vrsctl

(def count 0)

(defn increment (n)
  (set count (+ count n))
  (publish :count count))

(srv :counter :interface '(increment))
