#import "../../../components/index.typ": docs-category, scope

#show: docs-category.with(
  title: "Foundations",
  description: "Documentation for foundational definitions that make up the bedrock of Typst.",
  category: "foundations",
  scope-additions: (
    "none": type(none),
    "auto": type(auto),
  ),
  groups: (
    (
      name: "calc",
      def-target: calc,
      title: "Calculation",
      scope: scope(std, "calc"),
      definitions: dictionary(calc),
      description: "Documentation for the `calc` module, which contains definitions for mathematical computation.",
      docs: [
        Module for calculations and processing of numeric values.

        These definitions are part of the `calc` module and not imported by default.
      ],
    ),
    (
      name: "std",
      def-target: std,
      title: "Standard Library",
      definitions: (:),
      description: "Documentation for the `std` module, which contains all globally accessible items.",
      docs: [
        A module that contains all globally accessible items.

        = Using "shadowed" definitions <using-shadowed-definitions>
        The `std` module is useful whenever you overrode a name from the global scope (this is called _shadowing_). For instance, you might have used the name `text` for a parameter. To still access the `text` element, write `std.text`.

        ```example
        >>> #set page(margin: (left: 3em))
        #let par = [My special paragraph.]
        #let special(text) = {
          set std.text(style: "italic")
          set std.par.line(numbering: "1")
          text
        }

        #special(par)

        #lorem(10)
        ```

        = Conditional access <conditional-access>
        You can also use this in combination with the @dictionary.constructor[dictionary constructor] to conditionally access global definitions. This can, for instance, be useful to use new or experimental functionality when it is available, while falling back to an alternative implementation if used on an older Typst version. In particular, this allows us to create #link("https://en.wikipedia.org/wiki/Polyfill_(programming)")[polyfills].

        This can be as simple as creating an alias to prevent warning messages, for example, conditionally using `pattern` in Typst version 0.12, but using @tiling in newer versions. Since the parameters accepted by the `tiling` function match those of the older `pattern` function, using the `tiling` function when available and falling back to `pattern` otherwise will unify the usage across all versions. Note that, when creating a polyfill, @sys.version can also be very useful.

        ```typ
        #let tiling = if "tiling" in std { tiling } else { pattern }

        ...
        ```
      ],
    ),
    (
      name: "sys",
      def-target: sys,
      title: "System",
      scope: scope(std, "sys"),
      definitions: dictionary(sys),
      description: "Documentation for the `sys` module for system interactions.",
      docs: [
        Module for system interactions.
      ],
    ),
  ),
)

Foundational types and functions.

Here, you'll find documentation for basic data types like @int[integers] and @str[strings] as well as details about core computational functions.
