// Support for showing code blocks in one of three modes:
// - Source only via `source`
// - Preview only via `preview`
// - Source + Preview via `example`
//
// Cooperates with `docs/src/example.rs`.

#import "system.typ": colors, sizes
#import "base.typ": folding-details, labelled, small

// Shared styling properties for all elements with a "boxy" look.
#let radius = 2pt
#let padding = 0.9em
#let border = 0.5pt + colors.light-gray.shade-50

// A generic block with a boxy border.
#let boxy-block(..args) = block(
  stroke: border,
  radius: 2pt,
  clip: true,
  ..args,
)

// A block that is styled like an example.
#let example-like-block(..args) = boxy-block(
  width: 100%,
  inset: padding,
  fill: colors.genuine.white,
  ..args,
)

// Takes a raw block and processes `>>>` and `<<<` markers in them for display.
// The `>>>` marker indicates that a line is display-only while the `<<<` marker
// indices a compile-only line. This function applies the display part (trimming
// the former marker and stripping lines with the latter marker) while the code
// in `example.rs` deals with the markers in the opposite way.
#let with-hidden-lines(it) = {
  assert(it.at("lang", default: none) in (none, "typ", "example"))
  assert.eq(it.block, true)

  // Process the markers.
  let display = ""
  for line in it.text.split("\n") {
    if line.starts-with(">>>") {
      continue
    }

    if line.starts-with("<<< ") {
      line = line.slice(4)
    }

    display += line
    display += "\n"
  }

  // Applies the `<_stop>` label so that the reconstructed raw block is not
  // again affected by `source` show rules. A bit of a hack ...
  labelled(raw(display, lang: "typ", block: true), <_stop>)
}

// Nicely displays the raw block.
#let source(it) = {
  if it.at("label", default: none) == <_stop> {
    return it
  }

  context if target() == "paged" {
    set text(size: sizes.mono)
    example-like-block(if it.lang == "typ" {
      with-hidden-lines(it)
    } else {
      it
    })
  } else {
    it
  }
}

// Displays the rendering of a raw block containing Typst markup.
#let preview(it, ..args) = context {
  let pages = docs.compile-example(it, ..args)
  context if target() == "paged" {
    align(center, block(
      stroke: border,
      radius: radius,
      clip: true,
      stack(dir: ttb, spacing: 0.5em, ..pages),
    ))
  } else {
    html.div(class: "previewed-code", {
      html.div(class: "preview", pages.join())
    })
  }
}

// Displays a left/right or top/bottom (depending on content) side-by-side
// view of Typst markup and its paged output.
#let example(
  it,
  // Note: Must be `none` if `folding: false`.
  title: none,
  // Whether the example is folding in the HTML version. If `auto`, folds if the
  // example has a `title`.
  folding: auto,
  // Whether the folding example is open by default (vs collapsed).
  // Only applies of `folding` resolves to `true`.
  open: false,
  ..args,
) = context {
  show: rest => if target() == "paged" {
    if title != none {
      align(right, block(
        small[Example #sym.dot.c #text(weight: 500, title)],
        sticky: true,
        below: 0.65em,
      ))
    }
    rest
  } else {
    if folding == true or (folding == auto and title != none) {
      folding-details(
        title: [View example] + if title != none [: #title],
        open: open,
        rest,
      )
    } else {
      assert.eq(title, none)
      rest
    }
  }

  let pages = docs.compile-example(it, ..args)
  if target() == "paged" {
    layout(region => {
      let source = text(size: sizes.mono, with-hidden-lines(it))
      let available = region.width / 2 - 2 * padding.to-absolute()
      let wide = measure(source).width > available
      block(
        stroke: border,
        radius: radius,
        clip: true,
        grid(
          columns: if wide { 100% } else { (1fr, 1fr) },
          grid.cell(
            inset: padding,
            fill: colors.genuine.white,
            source,
          ),
          grid.cell(
            inset: padding,
            fill: colors.light-gray.shade-10,
            stroke: (
              (if wide { "top" } else { "left" }): border,
            ),
            align: center + horizon,
            stack(dir: ttb, spacing: 0.5em, ..pages),
          )
        ),
      )
    })
  } else {
    html.div(class: "previewed-code", {
      with-hidden-lines(it)
      html.div(class: "preview", pages.join())
    })
  }
}
