--- layout-in-fixed-size-block render ---
// Layout inside a block with certain dimensions should provide those dimensions.
#set page(height: 120pt)
#block(width: 60pt, height: 80pt, layout(size => [
  This block has a width of #size.width and height of #size.height
]))

--- layout-in-page-call render ---
// Layout without any container should provide the page's dimensions, minus its margins.
#page(width: 100pt, height: 100pt, {
  layout(size => [This page has a width of #size.width and height of #size.height ])
  h(1em)
  place(left, rect(width: 80pt, stroke: blue))
})
