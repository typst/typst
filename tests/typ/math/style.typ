#let part = $ a b A B $
#let kinds = (serif, sans, cal, frak, mono, bb)
#let modifiers = (v => v, ital, bold, v => ital(bold(v)))

#let cells = ([:triangle:nested:], [--], [`ital`], [`bold`], [both])
#for k in kinds {
  cells.push(raw(repr(k).trim("<function ").trim(">")))
  for m in modifiers {
    cells.push($ #m(#k(part)) $)
  }
}

#set page(width: auto)
#set par(align: center)
#table(columns: 1 + modifiers.len(), ..cells)
