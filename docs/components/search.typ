// Defines metadata for docs search in the web version.
//
// Cooperates with `docs/src/search.rs`.

#import "base.typ": labelled

// Defines one searchable entry (a search _index item_) in the documentation,
// i.e. one potential search hit.
//
// See `IndexItem` in the Rust sources for more details on the parameters.
#let register-index-item(
  category: none,
  kind: none,
  title: none,
  path: none,
  dest: none,
  keywords: (),
) = labelled(
  metadata((
    category: category,
    kind: kind,
    title: title,
    path: path,
    dest: dest,
    keywords: keywords,
  )),
  <metadata-index-item>,
)
