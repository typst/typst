#set text(size: 20pt)
#set page(width: auto)
测试字体 #lorem(5)

#text(stroke: 0.3pt + red)[测试字体#lorem(5)]

#text(stroke: 0.5pt + red)[测试字体#lorem(5)]

#text(stroke: 0.7pt + red)[测试字体#lorem(5)]

#text(stroke: 1pt + red)[测试字体#lorem(5)]

#text(stroke: 2pt + red)[测试字体#lorem(5)]

#text(stroke: 5pt + red)[测试字体#lorem(5)]

#text(stroke: 7pt + red)[测试字体#lorem(5)]

#text(stroke: (paint: blue, thickness: 1pt, dash: "dashed"))[测试字体#lorem(5)]

#text(stroke: 1pt + gradient.linear(..color.map.rainbow))[测试字体#lorem(5)] // gradient doesn't work now

---
// https://github.com/typst/typst/pull/2970#issuecomment-1864455231

#set page(width: auto)

#set text(size: 20pt, stroke: 1pt + red, fill: blue)
#lorem(3) 1pt + red stroke + blue fill

#set text(stroke: none)
#lorem(3) none + blue fill

#set text(stroke: none + black)
#lorem(3) none + black stroke + blue fill

#set text(stroke: 0pt)
#lorem(3) 0pt + blue fill
