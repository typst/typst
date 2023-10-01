// Test all gradient presets
---

#set page(width: 100pt, height: auto, margin: 0pt)
#set text(fill: white)
#set block(spacing: 0pt)

#let presets = (
    ("turbo", gradient.turbo()),
    ("cividis", gradient.cividis()),
    ("rainbow", gradient.rainbow()),
    ("spectral", gradient.spectral),
    ("viridis", gradient.viridis),
    ("inferno", gradient.inferno),
    ("magma", gradient.magma),
    ("plasma", gradient.plasma),
    ("rocket", gradient.rocket),
    ("mako", gradient.mako),
    ("vlag", gradient.vlag),
    ("icefire", gradient.icefire),
    ("flare", gradient.flare),
    ("crest", gradient.crest),
)

#for preset in presets {
    block(width: 100%, height: 10pt, fill: gradient.linear(..preset.at(1)))[
        #preset.at(0)
    ]
}