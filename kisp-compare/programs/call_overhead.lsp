; 20000 calls of a trivial function inside a WHILE loop
(defun id (x) x)
(setq i 20000)
(while (> i 0)
  (id i)
  (setq i (1- i)))
0
