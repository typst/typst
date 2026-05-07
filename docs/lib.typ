#import "components/index.typ" as components

// Renders the documentation.
#let docs(
  // The base path for the documentation's content files in the bundle. Only
  // applies to the website output format.
  //
  // Should be an absolute path like `/docs/` and *must* start and end with a
  // forward slash.
  content-base: "/",

  // The base path for the documentation's asset files in the bundle. Only
  // applies to the website output format.
  //
  // Should be an absolute path like `/assets/` and *must* start and end with a
  // forward slash.
  asset-base: "/assets/",

  // Additional content that is appended to the end of the main docs content.
  extra-chapters: none,
) = {
  assert(content-base.starts-with("/") and content-base.ends-with("/"))
  assert(asset-base.starts-with("/") and asset-base.ends-with("/"))

  // The config element and the `docs` function can be merged once we have
  // custom types.
  set stdx.config(
    content-base: content-base,
    asset-base: asset-base,
  )

  show: components.styling

  context if target() == "bundle" {
    include "assets/index.typ"
  }

  include "content/index.typ"
  extra-chapters
}
