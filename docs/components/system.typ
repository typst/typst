// Defines the design system.

// The subset of the brand colors that we use in the docs.
#let colors = (
  brand: rgb("#239dad"),
  text: (
    syntax: (
      teal: rgb("1d6c76"),
      blue: rgb("#4b69c6"),
    ),
  ),
  purple: (
    shade-10: rgb("f9dfff"),
  ),
  orange: (
    shade-10: rgb("ffedc1"),
  ),
  red: (
    shade-10: rgb("ffcbc4"),
  ),
  green: (
    shade-10: rgb("d1ffe2"),
  ),
  teal: (
    shade-20: rgb("a6ebe6"),
  ),
  blue: (
    shade-05: rgb("e8f9ff"),
    shade-10: rgb("a6eaff"),
    shade-50: rgb("007aff"),
    shade-80: rgb("001666"),
  ),
  dark-gray: (
    shade-05: rgb("666675"),
    shade-10: rgb("565565"),
    shade-60: rgb("2a2934"),
  ),
  light-gray: (
    shade-05: rgb("eff0f3"),
    shade-10: rgb("e4e5ea"),
    shade-30: rgb("caccd6"),
    shade-50: rgb("b0b3c2"),
    shade-70: rgb("979bad"),
  ),
  genuine: (
    white: rgb("fdfdfd"),
  ),
)

// Fonts that are used in the documentation.
#let fonts = (
  body: "HK Grotesk",
  mono: "Cascadia Mono",
  fallback: ("Noto Serif CJK SC",),
)

// Sizes that are used for fonts.
#let sizes = (
  body: 9pt,
  mono: 0.9em,
  small: 0.85em,
)

// The Typst logotype, in brand color.
#let logotype = {
  let src = str(stdx.read-dev-asset("typst.svg"))
  let colorized = src.replace("currentColor", colors.brand.to-hex())
  image(bytes(colorized))
}
