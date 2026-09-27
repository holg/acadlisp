; Odysseus-style workload: LLM-generated parametric drawing, 30x30 grid of
; circles with a short polar tick each (900 circles + 900 lines), then SVG export.
(setq n 30 i 0)
(while (< i n)
  (setq j 0)
  (while (< j n)
    (setq p (list (* i 20) (* j 20)))
    (command "CIRCLE" p 5)
    (command "LINE" p (polar p (* i 0.1) 8) "")
    (setq j (1+ j)))
  (setq i (1+ i)))
(* n n)
