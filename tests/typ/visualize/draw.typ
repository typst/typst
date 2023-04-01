// Test drawing.

---
// Some simple lines

#set page(height: 60pt)
#draw.path({
  draw.moveto(x: 20pt)
  draw.lineto(length: 40pt)
})
#draw.path(draw.lineto(x: 70%, y: 50%))

---
// Cubics and arcs
#stack(dir: ltr,
  box(stroke: 1pt + red, draw.path(closed: true, draw.cubicto(x1: 50pt, y2: 50pt))),
  h(10pt),
  draw.path(draw.cubicto(x1: 20pt, y1: 20pt, x2: 20pt, y2: 20pt, x: 40pt)),
)

#v(5pt)

#draw.path(draw.arcto(x: 50pt, r: 25pt))

#v(5pt)

#draw.path(draw.circle(x: 10pt, y: 10pt, r: 10pt))

---
// Filling and closing
#stack(dir: ltr,
  draw.path(fill: blue, {
    draw.circle(x: 20pt, y: 20pt, r: 20pt)
    draw.circle(x: 20pt, y: 20pt, r: 10pt, ccw: true)
  }),
  h(10pt),
  draw.path(fill: blue, {
    draw.circle(x: 20pt, y: 20pt, r: 20pt)
    draw.circle(x: 20pt, y: 20pt, r: 10pt, ccw: false)
  }),
)


#stack(dir: ltr,
  draw.path(fill: blue, {
    draw.lineto(dx: 10pt, dy: 10pt)
    draw.lineto(dx: 10pt, dy: -10pt)
  }),
  h(10pt),
  draw.path(fill: blue, closed: true, {
    draw.lineto(dx: 10pt, dy: 10pt)
    draw.lineto(dx: 10pt, dy: -10pt)
  }),
)

---
// Complex drawing

#draw.path(stroke: blue, fill: blue.lighten(80%), {
    import draw: *
    moveto(y: 10pt)
    for ms in (1, -1) {
        for sxy in ((1, -1), (-1, 1)) {
            let sx = sxy.at(0)
            let sy = sxy.at(1)
            let i = 0
            while i < 4 {
                lineto(dx: ms*sx*10pt, dy: ms*sy*10pt)
                lineto(dx: ms*10pt, dy: ms*10pt)
                i += 1
            }
        }
    }
    close()
    circle(x: 40pt, y: 50pt, r: 15pt, ccw: false)
})

---
// Test errors.

// Error: 16-21 point array must contain exactly two entries
#draw.arcto(r: (10,))

---
// Error: 12-14 expected path, found none
#draw.path({})
