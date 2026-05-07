// Exposes structured information about the standard library to the rest of the
// docs.
//
// Complements `docs/src/reflect.rs`.

// Maps from a representation of a value to all occurrances of it in the
// standard library. Can be used to efficiently determine the path to a
// particular value. Dictionaries cannot have arbitrary values as keys so we use
// a two-level repr lookup -> array search mechanism to avoid a full scan of the
// standard library.
#let std-map = {
  // These prelude definitions should not be preferred over the canonical
  // definitions in `std-path-of`.
  let excluded = (
    "luma",    // prefer `color.luma`
    "oklab",   // prefer `color.oklab`
    "oklch",   // prefer `color.oklch`
    "rgb",     // prefer `color.rgb`
    "cmyk",    // prefer `color.cmyk`
    "range",   // prefer `array.range`
    "pattern", // prefer `tiling`
  )

  let result = (:)
  let work = (
    ("", dictionary(std)),
    // Special values that have documentation, but no definition in std.
    ("", (
      "auto": type(auto),
      "none": type(none),
    )),
  )
  while work.len() > 0 {
    let (base, mod) = work.pop()
    for (name, value) in mod {
      let path = if base != "" { base + "." } + name
      if path in excluded {
        continue
      }

      let t = type(value)
      if t == module {
        work.push((path, dictionary(value)))
      } else if t == type or t == function {
        let info = stdx.describe(value)
        let scope = info.scope
        if scope != none {
          work.push((path, dictionary(scope)))
        }
        if t == type and info.constructor != none {
          work.push((path, (constructor: info.constructor)))
        }
      }

      let entry = (path, value)
      let r = repr(value)
      if r in result {
        result.at(r).push(entry)
      } else {
        result.insert(r, (entry,))
      }
    }
  }
  // Make std itself discoverable.
  result.insert(repr(std), (("std", std),))
  result
}

// Determines the canonical path to a value in the standard library.
//
// For instance
// - `std-path-of(heading) == "heading"`
// - `std-path-of(std.heading) == "heading"`
// - `std-path-of(math.cal) == "math.cal"`
// - `std-path-of(emoji.face) == "emoji.face"`
// - `{ let o = outline; std-path-of(o.entry) } == "outline.entry"`
#let std-path-of(value) = {
  for (path, option) in std-map.at(repr(value), default: ()) {
    if value == option {
      return path
    }
  }
  panic("failed to determine path to: " + repr(value))
}

// Defines the order and categories of types.
#let ty-categories = (
  kw: (type(none), type(auto)),
  num: (bool, int, float, length, angle, ratio, relative, fraction, decimal),
  col: (color, gradient, stroke),
  tiling: (tiling,),
  date: (datetime, duration),
  // TODO: Maybe `path` should be in a different one?
  str: (str, symbol, regex, path),
  meta: (label, selector, location),
  con: (content,),
  collect: (bytes, array, dictionary, arguments, version),
  fn: (function, type, module),
  layout: (alignment, direction),
)

// Maps from a type's repr to its category.
#let ty-category-map = {
  ty-categories
    .pairs()
    .map(((key, types)) => types.map(ty => (repr(ty), key)).to-dict())
    .join()
}

// Maps from a type's repr to its index in the canonical type ordering. (Used to
// sort type pills in the docs.)
#let ty-index-map = {
  ty-categories
    .values()
    .flatten()
    .map(repr)
    .enumerate()
    .map(((k, v)) => (v, k))
    .to-dict()
  ((repr("any")): 10000)
}

// Maps from a special type's repr to how it should be displayed. Other types
// use normal `repr`.
#let ty-name-map = (
  "\"any\"": "any",
  repr(type(auto)): "auto",
  repr(type(none)): "none",
)

// Turns a cast info into a flat array containing types and/or the string
// `"any"`.
#let flat-types(info) = {
  if info.kind == "union" {
    info
      .infos
      .map(flat-types)
      .flatten()
      .sorted(key: k => ty-index-map.at(repr(k)))
      .dedup()
  } else if info.kind == "type" {
    (info.ty,)
  } else if info.kind == "any" {
    ("any",)
  } else if info.kind == "value" and type(info.value) == str {
    (str,)
  } else {
    ()
  }
}

// Extracts all concrete strings and their details from a cast info.
#let cast-strings(info) = {
  if info.kind == "union" {
    info.infos.map(cast-strings).join()
  } else if info.kind == "value" and type(info.value) == str {
    ((info.value, info.details),)
  } else {
    ()
  }
}
