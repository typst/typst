#context asset(stdx.config.asset-base + "docs.css", read("docs.css"))

#context asset(
  stdx.config.asset-base + "base.css",
  read("base.css").replace(
    "url(\"/assets/fonts",
    "url(\"" + stdx.config.asset-base + "fonts",
  )
)

#context asset(
  stdx.config.asset-base + "docs.js",
  read("docs.js").replace(
    "const assetBase = \"/assets/\"",
    "const assetBase = \"" + stdx.config.asset-base + "\"",
  )
)

#let icons = (
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

#context for filename in icons {
  asset(
    stdx.config.asset-base + "icons/" + filename,
    stdx.read-dev-asset(filename),
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
  "NotoSansSymbols2-Regular.ttf",
  "NewCMMath-Regular.otf",
)

#context for filename in fonts {
  let (name, ext) = filename.split(".")
  let data = stdx.read-font(name)
  asset(stdx.config.asset-base + "fonts/" + filename, data)
}

// This metadata is emitted by images via a patched native show rule.
#context {
  let map = query(<metadata-asset>).map(meta => meta.value).to-dict()
  for (path, data) in map {
    asset(path, data)
  }
}

#context [
  #metadata(stdx.config.asset-base + "search.json") <search-index-path>
]
