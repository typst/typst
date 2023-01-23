#let part = $ a b A B $
#let kinds = (math.serif, math.sans, math.cal, math.frak, math.mono, math.bb)
#let modifiers = (v => v, math.italic, math.bold, v => math.italic(math.bold(v)))

#let cells = ([:triangle:nested:], [--], [`italic`], [`bold`], [both])
#for k in kinds {
  cells.push(raw(repr(k).trim("<function ").trim(">")))
  for m in modifiers {
    cells.push($ #m(#k(part)) $)
  }
}

#set page(width: auto)
#set align(center)
#table(columns: 1 + modifiers.len(), ..cells)
