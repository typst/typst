// Test all gradient presets
---

#set page(width: 100pt, height: auto, margin: 0pt)
#set block(spacing: 0pt)

#let presets = (
    gradient.turbo(),
    gradient.cividis(),
    gradient.rainbow(),
    gradient.spectral,
    gradient.viridis,
    gradient.inferno,
    gradient.magma,
    gradient.plasma,
    gradient.rocket,
    gradient.mako,
    gradient.vlag,
    gradient.icefire,
    gradient.flare,
    gradient.crest,
)

#for preset in presets {
    rect(width: 100%, height: 10pt, fill: gradient.linear(..preset))
}