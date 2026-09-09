// Test glyph outline extraction.

--- text-outlines-metadata paged empty ---
#context {
  let body = text(font: "DejaVu Sans", size: 10pt)[A B]
  let result = text.outlines(body)
  let measured = measure(body)
  test(result.width, measured.width)
  test(result.height, measured.height)
  assert(result.baseline > 0pt)
  test(result.glyphs.len(), 3)

  let (a, space, b) = result.glyphs
  test(a.text, "A")
  test(a.range, (0, 1))
  test(space.text, " ")
  test(space.range, (1, 2))
  test(space.components, none)
  test(b.text, "B")
  test(b.range, (2, 3))
  assert(a.components.len() > 0)
  assert(b.components.len() > 0)
  assert(a.advance.x > 0pt)
  test(a.advance.y, 0pt)
  assert(space.position.x > a.position.x)
  assert(b.position.x > space.position.x)
  test(a.position.y, result.baseline)
}

--- text-outlines-shaping paged empty ---
#context {
  let ligature = text.outlines(
    text(font: "DejaVu Sans", features: ("liga",))[ffi]
  )
  assert(ligature.glyphs.len() < 3)
  test(ligature.glyphs.first().text, "ffi")
  test(ligature.glyphs.first().range, (0, 3))

  let unjoined = text.outlines(
    text(font: "DejaVu Sans", features: (liga: 0))[ffi]
  )
  test(unjoined.glyphs.len(), 3)
}

--- text-outlines-fallback-and-bidi paged empty ---
#context {
  // Check fallback runs and cluster text.
  let fallback = text.outlines(
    text(font: ("Libertinus Serif", "Noto Sans Arabic"))[AبB]
  ).glyphs
  test(fallback.len(), 3)
  test(fallback.map(g => g.text).sorted(), ("A", "B", "ب"))
  assert(fallback.all(g => g.components != none))

  // RTL shaping uses visual glyph order and UTF-8 byte ranges.
  let rtl-glyphs = text.outlines(
    text(dir: rtl, font: "Noto Serif Hebrew")[אב]
  ).glyphs
  test(rtl-glyphs.len(), 2)
  test(rtl-glyphs.map(g => g.text), ("ב", "א"))
  test(rtl-glyphs.map(g => g.range), ((2, 4), (0, 2)))
  assert(rtl-glyphs.first().position.x < rtl-glyphs.last().position.x)

  // Mixed-direction text need not have monotonic x positions.
  let mixed = text.outlines(
    text(dir: rtl, font: ("Noto Serif Hebrew", "PT Sans"))[אבAB]
  ).glyphs
  test(mixed.len(), 4)
  assert(mixed.windows(2).any(pair =>
    pair.first().position.x > pair.last().position.x
  ))
}

--- text-outlines-non-outline-glyphs paged empty ---
#context {
  // Color and bitmap glyphs keep their metrics but have no monochrome outline.
  for font in ("Noto Color Emoji CBDT Subset", "Noto Color Emoji") {
    let glyphs = text.outlines(
      text(font: font, fallback: false)[✅]
    ).glyphs
    assert(glyphs.len() > 0)
    assert(glyphs.all(g => g.components == none))
    assert(glyphs.any(g => g.advance.x > 0pt))
  }

  // Cluster text trims edge ignorables but keeps interior joiners.
  let variation = text.outlines(
    text(font: "Noto Color Emoji", fallback: false)[♥️]
  ).glyphs.first()
  test(variation.text, "♥")
  let family = text.outlines(
    text(font: "Noto Color Emoji", fallback: false)[👩‍👩‍👦]
  ).glyphs.first()
  test(family.text, "👩‍👩‍👦")
}

--- text-outlines-long-multibyte-run paged empty ---
#context {
  // Exercise byte ranges past the old u16 limit without drawing outlines.
  let source = " " * 32769
  let glyphs = text.outlines(
    text(font: "DejaVu Sans", fallback: false, source)
  ).glyphs
  test(glyphs.len(), 32769)
  test(glyphs.last().text, " ")
  test(glyphs.last().range, (65536, 65538))
}

--- text-outlines-layout-and-transforms paged empty ---
#context {
  let wrapped = text.outlines(
    width: 25pt,
    text(font: "DejaVu Sans", size: 10pt)[AA AA],
  )
  assert(wrapped.glyphs.last().position.y > wrapped.glyphs.first().position.y)

  let plain = text.outlines(text(font: "DejaVu Sans")[A]).glyphs.first()
  let moved = text.outlines(
    move(dx: 7pt, dy: 3pt, text(font: "DejaVu Sans")[A])
  ).glyphs.first()
  test(moved.position.x - plain.position.x, 7pt)
  test(moved.position.y - plain.position.y, 3pt)
  let plain-start = plain.components.first().start
  let moved-start = moved.components.first().start
  test(moved-start.at(0) - plain-start.at(0), 7pt)
  test(moved-start.at(1) - plain-start.at(1), 3pt)

  let scaled = text.outlines(
    scale(x: 200%, origin: top + left, text(font: "DejaVu Sans")[A])
  ).glyphs.first()
  test(scaled.advance.x, 2 * plain.advance.x)
  test(scaled.advance.y, 2 * plain.advance.y)

  // Compare geometry relative to the glyph origin to isolate the affine map.
  let mirrored = text.outlines(
    scale(
      x: -100%,
      y: 50%,
      origin: top + left,
      text(font: "DejaVu Sans")[A],
    )
  ).glyphs.first()
  let local = plain.components.first().start
  let mirrored-local = mirrored.components.first().start
  test(
    mirrored-local.at(0) - mirrored.position.x,
    -(local.at(0) - plain.position.x),
  )
  test(
    mirrored-local.at(1) - mirrored.position.y,
    50% * (local.at(1) - plain.position.y),
  )

  // Translation changes positions, not advances.
  let transformed = text.outlines(
    move(
      dx: 11pt,
      dy: 13pt,
      rotate(
        90deg,
        origin: top + left,
        scale(x: 200%, y: 50%, text(font: "DejaVu Sans")[A]),
      ),
    )
  ).glyphs.first()
  assert(calc.abs(transformed.advance.x) < 0.001pt)
  test(calc.abs(transformed.advance.y), 2 * plain.advance.x)
}

--- text-outlines-variable-font paged empty ---
#context {
  let get(weight) = text.outlines(
    text(font: "Cantarell", variations: (wght: weight))[A]
  ).glyphs.first().components
  assert(get(100) != get(900))
}

--- text-outlines-components paged empty ---
#context {
  let glyph = text.outlines(text(font: "DejaVu Sans")[O]).glyphs.first()
  let funcs = glyph.components.map(it => it.func())
  test(funcs.first(), curve.move)
  assert(curve.cubic in funcs)
  test(funcs.last(), curve.close)
  test(glyph.components.last().mode, "straight")
}

--- text-outlines-render-and-modify paged ---
#set page(width: 180pt, height: 80pt, margin: 10pt)
#context {
  let result = text.outlines(text(font: "DejaVu Sans", size: 48pt)[Hi])
  let paths = result.glyphs.map(g => g.components).filter(it => it != none)
  let all = paths.flatten()

  // Outlines can be passed directly to `curve`.
  curve(fill: blue, ..all)

  // Curve components can be edited and rebuilt.
  let warp(point) = (
    point.at(0),
    point.at(1) + 4pt * calc.sin(point.at(0) / 3pt * 1rad),
  )
  let shifted = all.map(component => {
    let fields = component.fields()
    let func = component.func()
    if func == curve.move {
      curve.move(warp(fields.start))
    } else if func == curve.line {
      curve.line(warp(fields.end))
    } else if func == curve.cubic {
      curve.cubic(
        warp(fields.at("control-start")),
        warp(fields.at("control-end")),
        warp(fields.end),
      )
    } else {
      curve.close(mode: fields.mode)
    }
  })
  move(dx: 80pt, curve(fill: red, ..shifted))
}
