#set page(
  paper: "a8",
  margin: (x: 15pt, y: 30pt),
  header: {
    text(eastern)[*Typst*]
    h(1fr)
    text(0.8em)[_Chapter 1_]
  },
  footer: align(center)[\~ #counter(page).display() \~],
  background: counter(page).display(n => if n <= 2 {
    place(center + horizon, circle(radius: 1cm, fill: luma(90%)))
  })
)

But, soft! what light through yonder window breaks? It is the east, and Juliet
is the sun. Arise, fair sun, and kill the envious moon, Who is already sick and
pale with grief, That thou her maid art far more fair than she: Be not her maid,
since she is envious; Her vestal livery is but sick and green And none but fools
do wear it; cast it off. It is my lady, O, it is my love! O, that she knew she
were! She speaks yet she says nothing: what of that? Her eye discourses; I will
answer it.

#set page(header: none, height: auto, margin: (top: 15pt, bottom: 25pt))
The END.
