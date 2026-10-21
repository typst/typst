--- layout-in-fixed-size-block paged ---
// Layout inside a block with certain dimensions should provide those dimensions.
#set page(height: 120pt)
#block(width: 60pt, height: 80pt, layout(size => [
  This block has a width of #size.width and height of #size.height
]))

--- layout-in-page-call paged ---
// Layout without any container should provide the page's dimensions, minus its margins.
#page(width: 100pt, height: 100pt, {
  layout(size => [This page has a width of #size.width and height of #size.height ])
  h(1em)
  place(left, rect(width: 80pt, stroke: blue))
})

--- issue-6822-layout-expand paged ---
// Ensure that the body of `layout` can expand and respects alignment.
#rect(width: 100%, height: 50pt, layout(_ => {
  place(top + left)[1]
  place(center + horizon)[2]
  place(bottom + right)[3]
}))

--- issue-6822-layout-multiple paged ---
// But if there are multiple children (the `layout` is not the lone child), it
// cannot expand vertically anymore, just like `pad`.
#rect(width: 100%, height: 50pt, {
  pad(y: 1pt, align(center + horizon)[1])
  layout(_ =>  place(center + horizon)[2])
})
