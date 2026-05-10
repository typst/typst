#import "../../components/index.typ": icon, labelled, small

#let main-repo = "typst/typst"

#let chain-link(dest, title) = context {
  if target() == "paged" {
    link(dest, icon(16, "link", "Link"))
  } else {
    html.a(
      class: "chain",
      href: dest,
      title: title,
      icon(16, "link", "Link"),
    )
  }
}

#let hash-link(url, nr, repo) = {
  let body = {
    if repo != main-repo { repo }
    "#"
    str(nr)
  }
  small(labelled(link(url, body), <_stop>))
}

#let issue(nr, repo: main-repo) = context {
  let url = "https://github.com/" + repo + "/issues/" + str(nr)
  if target() == "paged" {
    hash-link(url, nr, repo)
  } else {
    chain-link(
      url,
      "Issue #" + str(nr) + " on " + repo,
    )
  }
}

#let pr(nr, repo: main-repo) = context {
  let url = "https://github.com/" + repo + "/pull/" + str(nr)
  if target() == "paged" {
    hash-link(url, nr, repo)
  } else {
    chain-link(
      url,
      "PR #" + str(nr) + " on " + repo,
    )
  }
}

#let gh(name) = link("https://github.com/" + name, [\@#name])
