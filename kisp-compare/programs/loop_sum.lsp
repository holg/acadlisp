; sum 1..50000 - AutoLISP idiom is WHILE + SETQ
(setq i 0 acc 0)
(while (< i 50000)
  (setq i (1+ i) acc (+ acc i)))
acc
