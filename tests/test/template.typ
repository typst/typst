#let case(id, size: 10pt, outset: 0pt) = {
  locate(loc => {
    let l = loc.position()
    record("(" + id + ": " + str(l.page) + ") ", loc)
    square(size: size, outset: outset)
  })
}