--- repr ---
#let t(a, b) = test(repr(a), b.text)

// Literal values.
#t(auto, `auto`)
#t(true, `true`)
#t(false, `false`)

// Numerical values.
#t(12.0, `12.0`)
#t(3.14, `3.14`)
#t(1234567890.0, `1234567890.0`)
#t(0123456789.0, `123456789.0`)
#t(0.0, `0.0`)
#t(-0.0, `-0.0`)
#t(-1.0, `-1.0`)
#t(-9876543210.0, `-9876543210.0`)
#t(-0987654321.0, `-987654321.0`)
#t(-3.14, `-3.14`)
#t(4.0 - 8.0, `-4.0`)
#t(float.inf, `float.inf`)
#t(-float.inf, `-float.inf`)
#t(float.nan, `float.nan`)

// Strings and escaping.
#t("hi", `"hi"`)
#t("a\n[]\"\u{1F680}string", `"a\n[]\"🚀string"`)

// Array and dictionary.
#t((1, 2, false, ), `(1, 2, false)`)
#t((a: 1, b: "2"), `(a: 1, b: "2")`)

// Functions.
#let f(x) = x
#t(f, `f`)
#t(rect , `rect`)
#t(() => none, `(..) => ..`)

// Types.
#t(int, `integer`)
#t(type("hi"), `string`)
#t(type((a: 1)), `dictionary`)

// Constants.
#t(ltr, `ltr`)
#t(left, `left`)

// Content.
#t([*Hey*], `strong(body: [Hey])`)
#t([A _sequence_], `sequence([A], [ ], emph(body: [sequence]))`)
#t([A _longer_ *sequence*!], ```
sequence(
  [A],
  [ ],
  emph(body: [longer]),
  [ ],
  strong(body: [sequence]),
  [!],
)
```)

// Colors and strokes.
#t(rgb("f7a205"), `rgb("#f7a205")`)
#t(2pt + rgb("f7a205"), `2pt + rgb("#f7a205")`)
#t(blue, `rgb("#0074d9")`)
#t(color.linear-rgb(blue), `color.linear-rgb(0%, 17.46%, 69.39%)`)
#t(oklab(blue), `oklab(56.22%, -0.05, -0.17)`)
#t(oklch(blue), `oklch(56.22%, 0.177, 253.71deg)`)
#t(cmyk(blue), `cmyk(100%, 46.54%, 0%, 14.9%)`)
#t(color.hsl(blue), `color.hsl(207.93deg, 100%, 42.55%)`)
#t(color.hsv(blue), `color.hsv(207.93deg, 100%, 85.1%)`)
#t(luma(blue), `luma(45.53%)`)

// Gradients.
#t(
  gradient.linear(blue, red),
  `gradient.linear((rgb("#0074d9"), 0%), (rgb("#ff4136"), 100%))`,
)
#t(
  gradient.linear(blue, red, dir: ttb),
  `gradient.linear(dir: rtl, (rgb("#0074d9"), 0%), (rgb("#ff4136"), 100%))`,
)
#t(
  gradient.linear(blue, red, relative: "self", angle: 45deg),
  `gradient.linear(angle: 45deg, relative: "self", (rgb("#0074d9"), 0%), (rgb("#ff4136"), 100%))`,
)
#t(
  gradient.linear(blue, red, space: rgb, angle: 45deg),
  `gradient.linear(angle: 45deg, space: rgb, (rgb("#0074d9"), 0%), (rgb("#ff4136"), 100%))`,
)
