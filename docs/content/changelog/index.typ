#import "../../components/index.typ": (
  docs-chapter, info, paged-heading-offset, section-outline,
)
#import "utils.typ": pr

// The single source of truth for release dates, in easily editable form.
#let releases = (
  "0.1.0":  "24.10.2025",
  "0.2.0":  "24.10.2025",
  "0.3.0":  "24.10.2025",
  "0.4.0":  "24.10.2025",
  "0.5.0":  "24.10.2025",
  "0.6.0":  "24.10.2025",
  "0.7.0":  "24.10.2025",
  "0.8.0":  "24.10.2025",
  "0.9.0":  "24.10.2025",
  "0.10.0": "24.10.2025",
  "0.11.0": "24.10.2025",
  "0.11.1": "24.10.2025",
  "0.12.0": "24.10.2025",
  "0.13.0": "24.10.2025",
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

// Displays avatars of contributors for a release.
#let contributors(contributors) = {
  [
    = Contributors <contributors>
    Thanks to everyone who contributed to this release!
  ]

  html.ul(class: "contribs", for contributor in contributors {
    html.li(html.a(
      href: "https://github.com/" + contributor.handle,
      target: "_blank",
      html.img(
        width: 64,
        height: 64,
        src: contributor.avatar,
        alt: "GitHub avatar of " + contributor.handle,
        title: {
          "@"
          contributor.handle
          " made "
          str(contributor.commits)
          " contribution"
          if contributor.commits > 1 { "s" }
        },
        crossorigin: "anonymous",
      ),
    ))
  })
}

#docs-chapter(
  title: "Changelog",
  route: "/changelog",
  description: "Learn what has changed in the latest Typst releases and move your documents forward.",
)[
  Learn what has changed in the latest Typst releases and move your documents forward. This section documents all changes to Typst since its initial public release.

  #context if target() == "paged" {
    info[
      Some changelog items contain references like this one: #pr(5017). These refer to a _pull request_ or _issue_ on #link("https://github.com")[GitHub] related to the change.
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

#context for (release, date) in releases {
  let s = str(release)
  docs-chapter(
    route: "/changelog/" + s,
    title: s,
    title-fmt: [Typst #s (#date.display("[month repr:long] [day], [year]"))],
    description: "Changes in Typst " + s,
    {
      set heading(outlined: false)
      include s + ".typ"

      // If information about contributors is provided via `sys.inputs`, render
      // it. The format should be like this:
      //
      // ```typc
      // sys.inputs.contributors == (
      //   "0.14.2": (
      //     (
      //       handle: "...",
      //       commits: 37,
      //       avatar: "https://avatars.githubusercontent.com/...",
      //     ),
      //     (
      //       handle: "...",
      //       commits: 17,
      //       avatar: "https://avatars.githubusercontent.com/...",
      //     ),
      //   )
      // )
      // ```
      //
      // For each version, it should list the GitHub handle, the number of
      // commits, and the URL of the user's GitHub avatar.
      context if target() == "html" and "contributors" in sys.inputs {
        let data = sys.inputs.contributors
        let s = str(release)
        if s in data {
          contributors(data.at(s))
        }
      }
    },
  )
}

#docs-chapter(
  title: "Earlier",
  title-fmt: [Changes in early, unversioned Typst],
  route: "/changelog/earlier",
  description: "Changes in early, unversioned Typst",
  {
    set heading(outlined: false)
    include "earlier.typ"
  },
)
