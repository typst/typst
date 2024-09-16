--- place-float-flow-around ---
#set page(height: 80pt)
#set place(float: true)
#place(bottom + center, rect(height: 20pt))
#lines(4)

--- place-float-queued ---
#set page(height: 180pt)
#set figure(placement: auto)

#figure(rect(height: 60pt), caption: [I])
#figure(rect(height: 40pt), caption: [II])
#figure(rect(), caption: [III])
#figure(rect(), caption: [IV])
A

--- place-float-align-auto ---
#set page(height: 140pt)
#set place(clearance: 5pt)
#set place(auto, float: true)

#place(rect[A])
#place(rect[B])
1 \ 2
#place(rect[C])
#place(rect[D])

--- place-float-in-column-align-auto ---
#set page(height: 150pt, columns: 2)
#set place(auto, float: true, clearance: 10pt)
#set rect(width: 75%)

#place(rect[I])
#place(rect[II])
#place(rect[III])
#place(rect[IV])

#lines(6)

#place(rect[V])

--- place-float-in-column-queued ---
#set page(height: 100pt, columns: 2)
#set place(float: true, clearance: 10pt)
#set rect(width: 75%)
#set text(costs: (widow: 0%, orphan: 0%))

#lines(3)

#place(top, rect[I])
#place(top, rect[II])
#place(bottom, rect[III])

#lines(3)

--- place-float-missing ---
// Error: 2-20 automatic positioning is only available for floating placement
// Hint: 2-20 you can enable floating placement with `place(float: true, ..)`
#place(auto)[Hello]

--- place-float-center-horizon ---
// Error: 2-45 floating placement must be `auto`, `top`, or `bottom`
#place(center + horizon, float: true)[Hello]

--- place-float-horizon ---
// Error: 2-36 floating placement must be `auto`, `top`, or `bottom`
#place(horizon, float: true)[Hello]

--- place-float-default ---
// Error: 2-27 floating placement must be `auto`, `top`, or `bottom`
#place(float: true)[Hello]

--- place-float-right ---
// Error: 2-34 floating placement must be `auto`, `top`, or `bottom`
#place(right, float: true)[Hello]

--- issue-2595-float-overlap ---
#set page(height: 80pt)

1
#place(auto, float: true, block(height: 100%, width: 100%, fill: aqua))
#place(auto, float: true, block(height: 100%, width: 100%, fill: red))
#lines(7)
