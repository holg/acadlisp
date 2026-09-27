; build a 1500 element list with cons, square it with mapcar, sum with apply
(setq lst nil i 1500)
(while (> i 0)
  (setq lst (cons i lst) i (1- i)))
(defun sq (x) (* x x))
(apply '+ (mapcar 'sq lst))
