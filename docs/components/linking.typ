// The documentation's cross-referencing system.
//
// Instead of directly referencing elements, we reference _definitions._ A
// definition target is either a label or a value from the standard library. In
// the second case, a canonical label is derived (see `std-path-of`).
//
// = Cross-references in the Typst documentation
// Across the documentation, the following convention is used for definition
// targets:
//
// - The docs for an std value are always referenced with its canonical std
//   path, e.g. `@figure`, `@calc.max`, `@math.attach`, `@str.codepoints`, or
//   `@outline.entry.indented`. Constructors are referenced with the special
//   `.constructor` path, e.g. `@int.constructor`.
// - A chapter is referenced with its HTML route, but replacing slashes with
//   `:`, e.g. `@guides:accessibility`.
// - A subheading in the docs for a value _or_ a chapter is referenced by taking
//   the label for the base and the label of the subheading and joining them
//   with `:`, e.g. `@heading:syntax` or `@guides:tables:column-sizes`.
// - A category is typically referenced via its route (e.g.
//   `@reference:foundations, but if it's tied to a definition, that takes
//   precedence (e.g. `@html` or `@math:function-calls`.
//
// A bare reference always renders the default `title` of the definition
// (monospaced path for std values, title for headings/chapters). If the
// reference has a supplement, it replaces the `title`.
//
// Compared to Typst's built-in referencing system, this custom one has two
// primary benefits:
// - We can attach a short, local, potentially duplicate labels to headings
//   (e.g. `<reading-order>`), also giving us a nice HTML id in the process, but
//   reference them with a fully qualified labels like
//   `<guides:accessibility:reading-order>`. In the future, something like this
//   might become possible with the built-in system, see
//   https://github.com/typst/typst/issues/7998 .
// - We can associate definitions with std values and let the system take care
//   of deriving canonical labels instead of having to build up complex labels
//   in the code that renders the relevant part of the std docs.
//
// = Registering definitions
// Definitions can be added anywhere in the docs with `register-def`.
// Frequently, this happens in heading show rules as that's where the relevant
// location is available. A definition consists of
// - a target (through which it can be referenced)
// - a destination (where it is defined); this is a location or, in HTML in some
//   cases, a route
// - a title (with which it a reference to it is displayed by default).
//
// = Retrieving definitions
// To retrieve a definition, you can use `def-metadata` (to get both the
// destination and the title) or `def-dest` (to get just the former). To link to
// a definition, you can use the `def` function.

#import "base.typ": labelled
#import "reflect.typ": std-path-of

// Returns the label with which a definition can be referenced in the docs.
//
// - If the `def-target` is a value, attempts to locate its path in the standard
//   library and returns the canonical label for it.
// - If the `def-target` is already a label, returns it unchanged.
#let def-label(def-target) = {
  if type(def-target) == label {
    def-target
  } else {
    label(std-path-of(def-target))
  }
}

// Retrieves the metadata for a definition.
//
// Returns a pair of a destination and a title. Requires context.
#let def-metadata(def-target) = {
  let def-label = def-label(def-target)
  let targets = query(selector.and(metadata, def-label))
  let len = targets.len()
  if len == 1 {
    targets.first().value
  } else if len == 0 {
    panic("found no definition for: " + repr(def-label))
  } else {
    panic({
      "found multiple definitions for: "
      repr(def-label)
      " ("
      targets
        .map(m => if type(m.value) == str { m.value } else { repr(m.value) })
        .join(", ")
      ")"
    })
  }
}

// Retrieves the linking destination for a definition.
#let def-dest(def-target) = def-metadata(def-target).first()

// Registers the destination under which a definition is documented.
#let register-def(
  // Either a label or a value that is defined in the standard library.
  def-target,
  // Where the target is documented.
  //
  // May be either a `location` or a route (the latter only in the HTML version
  // where both variants are used).
  dest,
  // What a reference to the target should display. Defaults to just the
  // resolved label's value in raw (e.g. `@int` => `` `int` ``), but could be
  // set to something else, like a heading's body (e.g.
  // `@introduction` => `[Introduction]`).
  title: auto,
) = {
  let def-label = def-label(def-target)
  if title == auto {
    title = raw(str(def-label))
  }
  labelled(metadata((dest, title)), def-label)
}
