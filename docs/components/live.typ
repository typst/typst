// Processes hot reloadable Typst documentation comments in Rust files.
//
// Cooperates with `docs/src/live.rs`.

#import "example.typ": example
#import "figure.typ": docs-figure
#import "table.typ": docs-table

// The definitions that are available in Rust doc comments.
#let scope = (
  example: example,
  docs-figure: docs-figure,
  docs-table: docs-table,
)

// Returns a dictionary with all live-loaded docs in the Rust file at the given
// path.
//
// This is a separate function so that it's memoized per path.
#let live-docs-at-path(path) = docs.docs-in-source(read(path))

// Processes docs content for a doc comment in the Rust sources.
//
// Takes the static docs markup and a def site key for hot reload. If the key is
// not `none`, attempts to load a live version of the docs markup from the Rust
// sources.
//
// In both cases, evaluates the markup.
#let live-docs(
  // The static docs content baked into the executable.
  markup,
  // The key for the native definition. May be `none`.
  //
  // See `DefSite::key` for more information.
  def-site,
) = {
  if def-site == none {
    return eval(markup, scope: scope, mode: "markup")
  }

  let path = "/" + def-site.path
  let live = live-docs-at-path(path)
  if def-site.key not in live {
    panic("def site was not found:", def-site)
  }

  let (markup, ranges) = live.at(def-site.key)
  docs.eval-mapped(
    markup,
    path,
    ranges,
    mode: "markup",
    scope: scope,
  )
}
