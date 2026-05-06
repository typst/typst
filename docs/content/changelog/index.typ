#import "../../components/index.typ": (
  docs-chapter, info, paged-heading-offset, section-outline,
)
#import "utils.typ": pr

// The single source of truth for release dates, in easily editable form.
#let releases = (
  "0.1.0":  "04.04.2023",
  "0.2.0":  "11.04.2023",
  "0.3.0":  "26.04.2023",
  "0.4.0":  "20.05.2023",
  "0.5.0":  "09.06.2023",
  "0.6.0":  "30.06.2023",
  "0.7.0":  "07.08.2023",
  "0.8.0":  "13.09.2023",
  "0.9.0":  "31.10.2023",
  "0.10.0": "04.12.2023",
  "0.11.0": "15.03.2024",
  "0.11.1": "17.05.2024",
  "0.12.0": "18.10.2024",
  "0.13.0": "19.02.2025",
  "0.13.1": "07.03.2025",
  "0.14.0": "24.10.2025",
  "0.14.1": "03.12.2025",
  "0.14.2": "12.12.2025",
)

// Converts the human-editable format above into typed (version, datetime)
// pairs.
#let releases = {
  releases
    .pairs()
    .map(((v, d)) => {
      let (day, month, year) = d.split(".").map(int)
      (
        version(..v.split(".").map(int)),
        datetime(day: day, month: month, year: year),
      )
    })
    .rev()
}

#docs-chapter(
  title: "Changelog",
  route: "/changelog",
  description: "Learn what has changed in the latest Typst releases and move your documents forward.",
)[
  Learn what has changed in the latest Typst releases and move your documents forward. This section documents all changes to Typst since its initial public release.

  #context if target() == "paged" {
    info[
      Some changelog items contain references like this one: #pr(5017). These refer to a _pull request_ or _issue_ on #link("https://github.com/typst/typst")[GitHub] related to the change. If the reference refers to another repository than `typst/typst`, the repository is explicitly listed like this: #pr(350, repo: "typst/hayagriva").
    ]
  }

  #section-outline(
    title: [Versions],
    label: <versions>,
  )
]

#show: paged-heading-offset.with(1)

// Since releases are already numbered, we don't want extra section numbering
// for them.
#show heading.where(level: 2): set heading(numbering: none)

#for (i, (release, date)) in releases.enumerate() {
  let s = str(release)
  docs-chapter(
    route: "/changelog/" + s,
    title: s,
    title-fmt: [Typst #s (#date.display("[month repr:long] [day], [year]"))],
    description: "Changes in Typst " + s,
    context {
      set heading(outlined: false) if target() == "paged"
      include s + ".typ"
      if target() == "html" [
        = Contributors <contributors>
        Thanks to everyone who contributed to this release!

        #block(html.elem("slot", attrs: (
          type: "contributors",
          from: "v" + str(releases.at(i + 1, default: ()).first(default: "23-03-28")),
          to: "v" + s,
        )))
      ]
    },
  )
}

#docs-chapter(
  title: "Earlier",
  title-fmt: [Changes in early, unversioned Typst],
  route: "/changelog/earlier",
  description: "Changes in early, unversioned Typst",
  context {
    set heading(outlined: false) if target() == "paged"
    include "earlier.typ"
  },
)
