#!/usr/bin/env vrsctl
# echo - An Echo Server

(loop
   (def req (recv))
   (let ((r (get req 0))
         (src (get req 1))
         (msg (get req 2)))
      (send src (list r msg))))
