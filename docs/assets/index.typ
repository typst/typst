#let assets = (
  "base.css",
  "docs.css",
  "docs.js",
  "12-tooltip.svg",
  "16-check.svg",
  "16-chevron-right.svg",
  "16-close.svg",
  "16-copy.svg",
  "16-docs-dark.svg",
  "16-hamburger-dark.svg",
  "16-link.svg",
  "16-search-gray.svg",
  "16-warn.svg",
  "32-reference-c.svg",
  "32-tutorial-c.svg",
)

#for filename in assets {
  asset(
    "assets/" + filename,
    read(filename, encoding: none),
  )
}

#let fonts = (
  "CascadiaMono-Regular.ttf",
  "CascadiaMono-Bold.ttf",
  "HKGrotesk-BlackItalic.ttf",
  "HKGrotesk-Bold.ttf",
  "HKGrotesk-BoldItalic.ttf",
  "HKGrotesk-ExtraBold.ttf",
  "HKGrotesk-ExtraBoldItalic.ttf",
  "HKGrotesk-ExtraLight.ttf",
  "HKGrotesk-ExtraLightItalic.ttf",
  "HKGrotesk-Italic.ttf",
  "HKGrotesk-Light.ttf",
  "HKGrotesk-LightItalic.ttf",
  "HKGrotesk-Medium.ttf",
  "HKGrotesk-MediumItalic.ttf",
  "HKGrotesk-Regular.ttf",
  "HKGrotesk-SemiBold.ttf",
  "HKGrotesk-SemiBoldItalic.ttf",
  "HKGrotesk-Thin.ttf",
)

#for path in fonts {
  asset(
    "assets/fonts/" + path,
    docs.read-dev-asset(path),
  )
}

// This metadata is emitted by images via a patched native show rule.
#context {
  let map = query(<metadata-asset>).map(meta => meta.value).to-dict()
  for (path, data) in map {
    asset(path, data)
  }
}
