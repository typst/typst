--- html-typed html ---
// String
#html.div(id: "hi")

// Different kinds of options.
#html.div(aria-autocomplete: none) // "none"
#html.div(aria-expanded: none) // "undefined"
#html.link(referrerpolicy: none) // present

// Different kinds of bools.
#html.div(autofocus: false) // absent
#html.div(autofocus: true) // present
#html.div(hidden: false) // absent
#html.div(hidden: true) // present
#html.div(aria-atomic: false) // "false"
#html.div(aria-atomic: true) // "true"
#html.div(translate: false) // "no"
#html.div(translate: true) // "yes"
#html.form(autocomplete: false) // "on"
#html.form(autocomplete: true) // "off"

// Char
#html.div(accesskey: "K")

// Int
#html.div(aria-colcount: 2)
#html.object(width: 120, height: 10)
#html.td(rowspan: 2)

// Float
#html.meter(low: 3.4, high: 7.9)

// Space-separated strings.
#html.div(class: "alpha")
#html.div(class: "alpha beta")
#html.div(class: ("alpha", "beta"))

// Comma-separated strings.
#html.div(html.input(accept: "image/jpeg"))
#html.div(html.input(accept: "image/jpeg, image/png"))
#html.div(html.input(accept: ("image/jpeg", "image/png")))

// Comma-separated floats.
#html.area(coords: (2.3, 4, 5.6))

// Colors.
#for c in (
  red,
  red.lighten(10%),
  luma(50%),
  cmyk(10%, 20%, 30%, 40%),
  oklab(27%, 20%, -3%, 50%),
  color.linear-rgb(20%, 30%, 40%, 50%),
  color.hsl(20deg, 10%, 20%),
  color.hsv(30deg, 20%, 30%),
) {
  html.link(color: c)
}

// Durations & datetimes.
#for d in (
  duration(weeks: 3, seconds: 4),
  duration(days: 1, minutes: 4),
  duration(),
  datetime(day: 10, month: 7, year: 2005),
  datetime(day: 1, month: 2, year: 0),
  datetime(hour: 6, minute: 30, second: 0),
  datetime(day: 1, month: 2, year: 0, hour: 11, minute: 11, second: 0),
  datetime(day: 1, month: 2, year: 0, hour: 6, minute: 0, second: 9),
) {
  html.div(html.time(datetime: d))
}

// Direction
#html.div(dir: ltr)[RTL]

// Image candidate and source size.
#html.img(
  src: "image.png",
  alt: "My wonderful image",
  srcset: (
    (src: "/image-120px.png", width: 120),
    (src: "/image-60px.png", width: 60),
  ),
  sizes: (
    (condition: "min-width: 800px", size: 400pt),
    (condition: "min-width: 400px", size: 250pt),
  )
)

// String enum.
#html.form(enctype: "text/plain")
#html.form(role: "complementary")
#html.div(hidden: "until-found")

// Or.
#html.div(aria-checked: false)
#html.div(aria-checked: true)
#html.div(aria-checked: "mixed")

// Input value.
#html.div(html.input(value: 5.6))
#html.div(html.input(value: red))
#html.div(html.input(min: 3, max: 9))

// Icon size.
#html.link(rel: "icon", sizes: ((32, 24), (64, 48)))

--- html-typed-dir-str html ---
// Error: 16-21 expected direction or auto, found string
#html.div(dir: "ltr")

--- html-typed-char-too-long html ---
// Error: 22-35 expected exactly one character
#html.div(accesskey: ("Ctrl", "K"))

--- html-typed-int-negative html ---
// Error: 18-21 number must be at least zero
#html.img(width: -10)

--- html-typed-int-zero html ---
// Error: 22-23 number must be positive
#html.textarea(rows: 0)

--- html-typed-float-negative html ---
// Error: 19-23 number must be positive
#html.input(step: -3.4)

--- html-typed-string-array-with-space html ---
// Error: 18-41 array item may not contain a space
// Hint: 18-41 the array attribute will be encoded as a space-separated string
#html.div(class: ("alpha beta", "gamma"))

--- html-typed-float-array-invalid-shorthand html ---
// Error: 20-23 expected array, found float
#html.area(coords: 4.5)

--- html-typed-dir-vertical html ---
// Error: 16-19 direction must be horizontal
#html.div(dir: ttb)

--- html-typed-string-enum-invalid html ---
// Error: 21-28 expected "application/x-www-form-urlencoded", "multipart/form-data", or "text/plain"
#html.form(enctype: "utf-8")

--- html-typed-or-invalid html ---
// Error: 25-31 expected boolean or "mixed"
#html.div(aria-checked: "nope")

--- html-typed-string-enum-or-array-invalid html ---
// Error: 27-33 expected array, "additions", "additions text", "all", "removals", or "text"
// Error: 49-54 expected boolean or "mixed"
#html.link(aria-relevant: "nope", aria-checked: "yes")

--- html-typed-srcset-both-width-and-density html ---
// Error: 19-64 cannot specify both `width` and `density`
#html.img(srcset: ((src: "img.png", width: 120, density: 0.5),))

--- html-typed-srcset-src-comma html ---
// Error: 19-50 `src` must not start or end with a comma
#html.img(srcset: ((src: "img.png,", width: 50),))

--- html-typed-sizes-string-size html ---
// Error: 18-66 expected length, found string
// Hint: 18-66 CSS lengths that are not expressible as Typst lengths are not yet supported
// Hint: 18-66 you can use `html.elem` to create a raw attribute
#html.img(sizes: ((condition: "min-width: 100px", size: "10px"),))

--- html-typed-input-value-invalid html ---
// Error: 20-25 expected string, float, datetime, color, or array, found boolean
#html.input(value: false)

--- html-typed-input-bound-invalid html ---
// Error: 18-21 expected string, float, or datetime, found color
#html.input(min: red)

--- html-typed-icon-size-invalid html ---
// Error: 32-45 expected array, found string
#html.link(rel: "icon", sizes: "10x20 20x30")

--- html-typed-hidden-none html ---
// Error: 19-23 expected boolean or "until-found", found none
#html.div(hidden: none)

--- html-typed-invalid-body html ---
// Error: 10-14 unexpected argument
#html.img[hi]
